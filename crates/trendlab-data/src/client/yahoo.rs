//! Yahoo Finance client for fetching OHLCV data.

use crate::error::{DataError, DataResult};
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// Yahoo Finance API client with rate limiting.
pub struct YahooClient {
    client: Client,
    base_url: String,
}

impl Default for YahooClient {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooClient {
    /// Create a new Yahoo Finance client.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: "https://query1.finance.yahoo.com".to_string(),
        }
    }

    /// Fetch OHLCV data for a symbol within a date range.
    ///
    /// Returns raw OHLCV rows suitable for conversion to `Bar` or Polars DataFrame.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<Vec<OhlcvRow>> {
        if start > end {
            return Err(DataError::InvalidDateRange { start, end });
        }

        let period1 = date_to_unix(start);
        let period2 = date_to_unix(end) + 86400; // Include end date

        let url = format!(
            "{}/v8/finance/chart/{}?period1={}&period2={}&interval=1d&events=history",
            self.base_url, symbol, period1, period2
        );

        debug!(url = %url, "Fetching Yahoo Finance data");

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(DataError::RateLimited { retry_after_secs: 60 });
        }

        if !response.status().is_success() {
            return Err(DataError::YahooFetch {
                symbol: symbol.to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        let json: YahooResponse = response.json().await.map_err(|e| DataError::Parse {
            symbol: symbol.to_string(),
            message: e.to_string(),
        })?;

        parse_yahoo_response(symbol, json)
    }
}

/// A single OHLCV row from Yahoo Finance.
#[derive(Debug, Clone)]
pub struct OhlcvRow {
    /// Trading date.
    pub date: NaiveDate,
    /// Opening price.
    pub open: f64,
    /// Highest price.
    pub high: f64,
    /// Lowest price.
    pub low: f64,
    /// Closing price (adjusted).
    pub close: f64,
    /// Trading volume.
    pub volume: u64,
}

// Yahoo Finance API response structures

#[derive(Deserialize)]
struct YahooResponse {
    chart: ChartResult,
}

#[derive(Deserialize)]
struct ChartResult {
    result: Option<Vec<ChartData>>,
    error: Option<YahooError>,
}

#[derive(Deserialize)]
struct YahooError {
    code: String,
    description: String,
}

#[derive(Deserialize)]
struct ChartData {
    timestamp: Option<Vec<i64>>,
    indicators: Indicators,
}

#[derive(Deserialize)]
struct Indicators {
    quote: Vec<Quote>,
    adjclose: Option<Vec<AdjClose>>,
}

#[derive(Deserialize)]
struct Quote {
    open: Vec<Option<f64>>,
    high: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    volume: Vec<Option<u64>>,
}

#[derive(Deserialize)]
struct AdjClose {
    adjclose: Vec<Option<f64>>,
}

fn date_to_unix(date: NaiveDate) -> i64 {
    let datetime = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    Utc.from_utc_datetime(&datetime).timestamp()
}

fn unix_to_date(ts: i64) -> NaiveDate {
    chrono::DateTime::from_timestamp(ts, 0)
        .unwrap_or_else(|| Utc::now())
        .date_naive()
}

fn parse_yahoo_response(symbol: &str, response: YahooResponse) -> DataResult<Vec<OhlcvRow>> {
    // Check for API error
    if let Some(err) = response.chart.error {
        return Err(DataError::YahooFetch {
            symbol: symbol.to_string(),
            message: format!("{}: {}", err.code, err.description),
        });
    }

    let result = response
        .chart
        .result
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| DataError::NoData {
            symbol: symbol.to_string(),
        })?;

    let timestamps = result.timestamp.ok_or_else(|| DataError::NoData {
        symbol: symbol.to_string(),
    })?;

    let quote = result
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or_else(|| DataError::Parse {
            symbol: symbol.to_string(),
            message: "Missing quote data".to_string(),
        })?;

    // Use adjusted close if available, otherwise regular close
    let adj_close = result
        .indicators
        .adjclose
        .and_then(|ac| ac.into_iter().next())
        .map(|ac| ac.adjclose);

    let mut rows = Vec::with_capacity(timestamps.len());
    let mut skipped = 0;

    for (i, ts) in timestamps.into_iter().enumerate() {
        // Get values, skip if any are missing
        let open = match quote.open.get(i).copied().flatten() {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };
        let high = match quote.high.get(i).copied().flatten() {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };
        let low = match quote.low.get(i).copied().flatten() {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };
        let close = match adj_close
            .as_ref()
            .and_then(|ac| ac.get(i).copied().flatten())
            .or_else(|| quote.close.get(i).copied().flatten())
        {
            Some(v) => v,
            None => {
                skipped += 1;
                continue;
            }
        };
        let volume = quote.volume.get(i).copied().flatten().unwrap_or(0);

        rows.push(OhlcvRow {
            date: unix_to_date(ts),
            open,
            high,
            low,
            close,
            volume,
        });
    }

    if skipped > 0 {
        warn!(symbol = %symbol, skipped = skipped, "Skipped rows with missing data");
    }

    if rows.is_empty() {
        return Err(DataError::NoData {
            symbol: symbol.to_string(),
        });
    }

    // Sort by date ascending
    rows.sort_by_key(|r| r.date);

    debug!(symbol = %symbol, rows = rows.len(), "Fetched OHLCV data");

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_conversion() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let unix = date_to_unix(date);
        let back = unix_to_date(unix);
        assert_eq!(date, back);
    }

    #[test]
    fn test_invalid_date_range() {
        let client = YahooClient::new();
        let start = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.fetch_ohlcv("AAPL", start, end));

        assert!(matches!(result, Err(DataError::InvalidDateRange { .. })));
    }
}
