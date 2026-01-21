//! Data-specific error types.

use thiserror::Error;

/// Errors that can occur during data operations.
#[derive(Error, Debug)]
pub enum DataError {
    /// Failed to fetch data from Yahoo Finance.
    #[error("Yahoo Finance fetch failed for {symbol}: {message}")]
    YahooFetch {
        /// The symbol that failed to fetch.
        symbol: String,
        /// Error message.
        message: String,
    },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse response.
    #[error("Parse error for {symbol}: {message}")]
    Parse {
        /// The symbol being parsed.
        symbol: String,
        /// Error message.
        message: String,
    },

    /// Cache I/O error.
    #[error("Cache I/O error: {0}")]
    CacheIo(#[from] std::io::Error),

    /// Parquet read/write error.
    #[error("Parquet error: {0}")]
    Parquet(#[from] polars::error::PolarsError),

    /// Symbol not found in universe.
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Invalid date range.
    #[error("Invalid date range: {start} to {end}")]
    InvalidDateRange {
        /// Start date.
        start: chrono::NaiveDate,
        /// End date.
        end: chrono::NaiveDate,
    },

    /// No data available for the requested range.
    #[error("No data available for {symbol} in requested range")]
    NoData {
        /// The symbol with no data.
        symbol: String,
    },

    /// Cache is stale and refresh failed.
    #[error("Cache stale for {symbol}, refresh failed: {reason}")]
    CacheStale {
        /// The symbol with stale cache.
        symbol: String,
        /// Reason for refresh failure.
        reason: String,
    },
}

/// Result type alias for data operations.
pub type DataResult<T> = Result<T, DataError>;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_error_display() {
        let err = DataError::YahooFetch {
            symbol: "AAPL".into(),
            message: "connection timeout".into(),
        };
        assert!(err.to_string().contains("AAPL"));
        assert!(err.to_string().contains("connection timeout"));
    }

    #[test]
    fn test_rate_limited_display() {
        let err = DataError::RateLimited { retry_after_secs: 60 };
        assert!(err.to_string().contains("60"));
    }

    #[test]
    fn test_invalid_date_range() {
        let err = DataError::InvalidDateRange {
            start: NaiveDate::from_ymd_opt(2024, 12, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        assert!(err.to_string().contains("2024-12-01"));
    }
}
