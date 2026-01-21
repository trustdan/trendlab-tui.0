//! Parquet-based data cache with ZSTD compression.

use crate::client::{OhlcvRow, YahooClient};
use crate::error::{DataError, DataResult};
use chrono::NaiveDate;
use polars::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument};

/// Parquet cache for OHLCV data.
///
/// # Design
/// - One Parquet file per symbol
/// - ZSTD compression for storage efficiency
/// - Delta sync: only fetch missing dates
/// - HashSet-based date deduplication
pub struct ParquetCache {
    cache_dir: PathBuf,
    client: YahooClient,
}

impl ParquetCache {
    /// Create a new Parquet cache at the given directory.
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> DataResult<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            client: YahooClient::new(),
        })
    }

    /// Get the cache file path for a symbol.
    fn cache_path(&self, symbol: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.parquet", symbol.to_uppercase()))
    }

    /// Load cached data for a symbol, if it exists.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub fn load(&self, symbol: &str) -> DataResult<Option<LazyFrame>> {
        let path = self.cache_path(symbol);

        if !path.exists() {
            debug!("Cache miss for {}", symbol);
            return Ok(None);
        }

        debug!("Cache hit for {}", symbol);
        let lf = LazyFrame::scan_parquet(&path, Default::default())?;
        Ok(Some(lf))
    }

    /// Get cached date range for a symbol.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub fn cached_range(&self, symbol: &str) -> DataResult<Option<(NaiveDate, NaiveDate)>> {
        let path = self.cache_path(symbol);

        if !path.exists() {
            return Ok(None);
        }

        let df = LazyFrame::scan_parquet(&path, Default::default())?
            .select([col("date").min().alias("min"), col("date").max().alias("max")])
            .collect()?;

        let min_date = df
            .column("min")?
            .date()?
            .get(0)
            .map(|d| date32_to_naive(d));
        let max_date = df
            .column("max")?
            .date()?
            .get(0)
            .map(|d| date32_to_naive(d));

        match (min_date, max_date) {
            (Some(min), Some(max)) => Ok(Some((min, max))),
            _ => Ok(None),
        }
    }

    /// Fetch and cache data for a symbol within a date range.
    ///
    /// Uses delta sync: only fetches dates not already in cache.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn fetch_and_cache(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<LazyFrame> {
        // Load existing cached dates
        let existing_dates = self.load_cached_dates(symbol)?;

        // Determine what needs to be fetched
        let (fetch_start, fetch_end) = if existing_dates.is_empty() {
            (start, end)
        } else {
            // Find gaps in the cache
            let cached_min = existing_dates.iter().min().copied().unwrap();
            let cached_max = existing_dates.iter().max().copied().unwrap();

            if start >= cached_min && end <= cached_max {
                // Fully cached, check for gaps
                let mut missing = Vec::new();
                let mut d = start;
                while d <= end {
                    if !existing_dates.contains(&d) {
                        missing.push(d);
                    }
                    d = d.succ_opt().unwrap_or(d);
                }

                if missing.is_empty() {
                    info!("Using cached data for {} (fully covered)", symbol);
                    return self.load(symbol)?.ok_or(DataError::NoData {
                        symbol: symbol.to_string(),
                    });
                }

                // Fetch the missing range
                (*missing.iter().min().unwrap(), *missing.iter().max().unwrap())
            } else {
                // Need to extend cache
                (start.min(cached_min), end.max(cached_max))
            }
        };

        info!(
            "Fetching {} from {} to {}",
            symbol, fetch_start, fetch_end
        );

        // Fetch new data
        let new_rows = self.client.fetch_ohlcv(symbol, fetch_start, fetch_end).await?;

        // Merge with existing cache
        let merged = self.merge_and_save(symbol, new_rows, &existing_dates)?;

        // Return filtered to requested range
        Ok(merged.filter(
            col("date")
                .gt_eq(lit(naive_to_date32(start)))
                .and(col("date").lt_eq(lit(naive_to_date32(end)))),
        ))
    }

    /// Load all cached dates for deduplication.
    fn load_cached_dates(&self, symbol: &str) -> DataResult<HashSet<NaiveDate>> {
        let path = self.cache_path(symbol);

        if !path.exists() {
            return Ok(HashSet::new());
        }

        let df = LazyFrame::scan_parquet(&path, Default::default())?
            .select([col("date")])
            .collect()?;

        let dates: HashSet<NaiveDate> = df
            .column("date")?
            .date()?
            .into_iter()
            .flatten()
            .map(date32_to_naive)
            .collect();

        Ok(dates)
    }

    /// Merge new rows with cache and save.
    fn merge_and_save(
        &self,
        symbol: &str,
        new_rows: Vec<OhlcvRow>,
        existing_dates: &HashSet<NaiveDate>,
    ) -> DataResult<LazyFrame> {
        let path = self.cache_path(symbol);

        // Filter out duplicates
        let new_rows: Vec<_> = new_rows
            .into_iter()
            .filter(|r| !existing_dates.contains(&r.date))
            .collect();

        // Convert new rows to DataFrame
        let new_df = rows_to_dataframe(&new_rows)?;

        // Merge with existing if present
        let merged_df = if path.exists() {
            let existing = LazyFrame::scan_parquet(&path, Default::default())?.collect()?;
            concat([existing.lazy(), new_df.lazy()], Default::default())?.collect()?
        } else {
            new_df
        };

        // Sort by date
        let sorted = merged_df.sort(["date"], Default::default())?;

        // Write to parquet with ZSTD compression
        let mut file = std::fs::File::create(&path)?;
        ParquetWriter::new(&mut file)
            .with_compression(ParquetCompression::Zstd(None))
            .finish(&mut sorted.clone())?;

        debug!("Saved {} rows to cache for {}", sorted.height(), symbol);

        Ok(sorted.lazy())
    }

    /// Clear cache for a specific symbol.
    pub fn clear(&self, symbol: &str) -> DataResult<()> {
        let path = self.cache_path(symbol);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Clear entire cache.
    pub fn clear_all(&self) -> DataResult<()> {
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|ext| ext == "parquet") {
                std::fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}

/// Convert OHLCV rows to a Polars DataFrame.
fn rows_to_dataframe(rows: &[OhlcvRow]) -> DataResult<DataFrame> {
    let dates: Vec<i32> = rows.iter().map(|r| naive_to_date32(r.date)).collect();
    let opens: Vec<f64> = rows.iter().map(|r| r.open).collect();
    let highs: Vec<f64> = rows.iter().map(|r| r.high).collect();
    let lows: Vec<f64> = rows.iter().map(|r| r.low).collect();
    let closes: Vec<f64> = rows.iter().map(|r| r.close).collect();
    let volumes: Vec<u64> = rows.iter().map(|r| r.volume).collect();

    let df = DataFrame::new(vec![
        Column::new("date".into(), dates).cast(&DataType::Date)?,
        Column::new("open".into(), opens),
        Column::new("high".into(), highs),
        Column::new("low".into(), lows),
        Column::new("close".into(), closes),
        Column::new("volume".into(), volumes),
    ])?;

    Ok(df)
}

/// Convert NaiveDate to i32 (days since epoch).
fn naive_to_date32(date: NaiveDate) -> i32 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    (date - epoch).num_days() as i32
}

/// Convert i32 (days since epoch) to NaiveDate.
fn date32_to_naive(days: i32) -> NaiveDate {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    epoch + chrono::Duration::days(days as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_conversion() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let days = naive_to_date32(date);
        let back = date32_to_naive(days);
        assert_eq!(date, back);
    }

    #[test]
    fn test_rows_to_dataframe() {
        let rows = vec![
            OhlcvRow {
                date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                open: 100.0,
                high: 105.0,
                low: 98.0,
                close: 103.0,
                volume: 1000,
            },
            OhlcvRow {
                date: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                open: 103.0,
                high: 108.0,
                low: 102.0,
                close: 107.0,
                volume: 1500,
            },
        ];

        let df = rows_to_dataframe(&rows).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 6);
    }
}
