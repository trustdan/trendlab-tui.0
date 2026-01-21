//! TrendLab Data
//!
//! Invisible data infrastructure: fetching, caching, and universe management.
//!
//! # Responsibilities
//!
//! - Fetch OHLCV data from Yahoo Finance
//! - Cache in Parquet format with ZSTD compression
//! - Incremental updates (delta fetch)
//! - Symbol universe management
//! - Pre-compute common indicators (ATR, ADX)
//!
//! # Design Principle
//!
//! Data should be invisible. The engine requests bars, this crate provides them.
//! No UI, no user interaction, just reliable data.
//!
//! # Example
//!
//! ```ignore
//! use trendlab_data::{DataProvider, UniverseId};
//!
//! let provider = DataProvider::new("./cache")?;
//! let universe = UniverseId::Us30.get();
//!
//! for symbol in universe.iter() {
//!     let data = provider.get_data(symbol, start, end).await?;
//!     // data is a LazyFrame with OHLCV + optional indicators
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

mod cache;
mod client;
pub mod error;
pub mod indicators;
mod universe;

pub use cache::ParquetCache;
pub use client::{OhlcvRow, YahooClient};
pub use error::{DataError, DataResult};
pub use universe::{Universe, UniverseId};

use chrono::NaiveDate;
use polars::prelude::*;
use std::path::Path;
use trendlab_core::types::Bar;
use tracing::{info, instrument};

/// High-level data provider that unifies caching, fetching, and indicators.
///
/// This is the main entry point for data access. It handles:
/// - Automatic caching with delta sync
/// - Lazy indicator computation
/// - Conversion to `Bar` sequences
pub struct DataProvider {
    cache: ParquetCache,
}

impl DataProvider {
    /// Create a new DataProvider with the given cache directory.
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> DataResult<Self> {
        Ok(Self {
            cache: ParquetCache::new(cache_dir)?,
        })
    }

    /// Get OHLCV data for a symbol within a date range.
    ///
    /// Returns a LazyFrame for efficient processing.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_data(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<LazyFrame> {
        self.cache.fetch_and_cache(symbol, start, end).await
    }

    /// Get OHLCV data with common indicators pre-computed.
    ///
    /// Adds ATR(14), Donchian(20), ADX(14) by default.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_data_with_indicators(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<LazyFrame> {
        let lf = self.get_data(symbol, start, end).await?;

        let lf = indicators::add_atr(lf, 14);
        let lf = indicators::add_donchian(lf, 20);
        let lf = indicators::add_adx(lf, 14);

        Ok(lf)
    }

    /// Get OHLCV data converted to Bar sequence.
    ///
    /// Collects the LazyFrame and converts to a Vec<Bar>.
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_bars(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<Vec<Bar>> {
        let df = self.get_data(symbol, start, end).await?.collect()?;
        dataframe_to_bars(&df)
    }

    /// Prefetch data for all symbols in a universe.
    ///
    /// Useful for warming up the cache before backtesting.
    #[instrument(skip(self), fields(universe = %universe.name))]
    pub async fn prefetch_universe(
        &self,
        universe: &Universe,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DataResult<()> {
        info!(
            "Prefetching {} symbols from {} to {}",
            universe.len(),
            start,
            end
        );

        for symbol in universe.iter() {
            if let Err(e) = self.cache.fetch_and_cache(symbol, start, end).await {
                tracing::warn!(symbol = %symbol, error = %e, "Failed to fetch symbol");
            }
        }

        Ok(())
    }

    /// Clear cache for a specific symbol.
    pub fn clear_cache(&self, symbol: &str) -> DataResult<()> {
        self.cache.clear(symbol)
    }

    /// Clear entire cache.
    pub fn clear_all_cache(&self) -> DataResult<()> {
        self.cache.clear_all()
    }

    /// Get cached date range for a symbol.
    pub fn cached_range(&self, symbol: &str) -> DataResult<Option<(NaiveDate, NaiveDate)>> {
        self.cache.cached_range(symbol)
    }
}

/// Convert a Polars DataFrame to a Vec<Bar>.
fn dataframe_to_bars(df: &DataFrame) -> DataResult<Vec<Bar>> {
    let dates = df.column("date")?.date()?;
    let opens = df.column("open")?.f64()?;
    let highs = df.column("high")?.f64()?;
    let lows = df.column("low")?.f64()?;
    let closes = df.column("close")?.f64()?;
    let volumes = df.column("volume")?.u64()?;

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    let bars: Vec<Bar> = (0..df.height())
        .filter_map(|i| {
            let date_days = dates.get(i)?;
            let date = epoch + chrono::Duration::days(date_days as i64);
            let open = opens.get(i)?;
            let high = highs.get(i)?;
            let low = lows.get(i)?;
            let close = closes.get(i)?;
            let volume = volumes.get(i).unwrap_or(0);

            Some(Bar::new(date, open, high, low, close, volume, i))
        })
        .collect();

    Ok(bars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universe_exports() {
        let universe = UniverseId::Test.get();
        assert!(!universe.is_empty());
        assert!(universe.contains("AAPL"));
    }

    #[test]
    fn test_dataframe_to_bars() {
        let dates: Vec<i32> = vec![19724, 19725, 19726]; // 2024-01-02, 03, 04
        let opens = vec![100.0, 101.0, 102.0];
        let highs = vec![105.0, 106.0, 107.0];
        let lows = vec![98.0, 99.0, 100.0];
        let closes = vec![103.0, 104.0, 105.0];
        let volumes: Vec<u64> = vec![1000, 2000, 3000];

        let df = DataFrame::new(vec![
            Column::new("date".into(), dates).cast(&DataType::Date).unwrap(),
            Column::new("open".into(), opens),
            Column::new("high".into(), highs),
            Column::new("low".into(), lows),
            Column::new("close".into(), closes),
            Column::new("volume".into(), volumes),
        ])
        .unwrap();

        let bars = dataframe_to_bars(&df).unwrap();
        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0].open, 100.0);
        assert_eq!(bars[0].idx, 0);
        assert_eq!(bars[2].close, 105.0);
    }
}
