//! Integration tests for trendlab-data.
//!
//! Note: Some tests require network access and are marked with #[ignore].
//! Run with `cargo test --package trendlab-data -- --ignored` to include them.

use chrono::NaiveDate;
use polars::prelude::*;
use std::collections::HashSet;
use tempfile::tempdir;
use trendlab_data::{DataProvider, Universe, UniverseId};

/// Test that universes contain expected symbols.
#[test]
fn test_universe_contents() {
    let us30 = UniverseId::Us30.get();
    assert_eq!(us30.len(), 30);
    assert!(us30.contains("AAPL"));
    assert!(us30.contains("MSFT"));

    let sp100 = UniverseId::Sp100.get();
    assert!(sp100.len() >= 90);

    // No duplicates
    let symbols: HashSet<_> = us30.iter().collect();
    assert_eq!(symbols.len(), us30.len());
}

/// Test custom universe creation.
#[test]
fn test_custom_universe() {
    let custom = Universe::custom("MyUniverse", vec!["FOO".into(), "BAR".into(), "BAZ".into()]);
    assert_eq!(custom.name, "MyUniverse");
    assert_eq!(custom.len(), 3);
    assert!(custom.contains("foo")); // case insensitive
}

/// Test single symbol universe.
#[test]
fn test_single_universe() {
    let single = Universe::single("TSLA");
    assert_eq!(single.len(), 1);
    assert!(single.name.contains("TSLA"));
}

/// Test indicator functions with synthetic data.
#[test]
fn test_indicators_atr() {
    let lf = create_sample_lazyframe(100);
    let result = trendlab_data::indicators::add_atr(lf, 14).collect().unwrap();

    assert!(result.column("atr_14").is_ok());

    // ATR should be positive where valid
    let atr = result.column("atr_14").unwrap().f64().unwrap();
    for i in 14..result.height() {
        if let Some(val) = atr.get(i) {
            assert!(val > 0.0, "ATR should be positive at index {}", i);
        }
    }
}

#[test]
fn test_indicators_donchian() {
    let lf = create_sample_lazyframe(50);
    let result = trendlab_data::indicators::add_donchian(lf, 20).collect().unwrap();

    assert!(result.column("donchian_high_20").is_ok());
    assert!(result.column("donchian_low_20").is_ok());
    assert!(result.column("donchian_mid_20").is_ok());

    // Verify mid is average of high and low
    let high = result.column("donchian_high_20").unwrap().f64().unwrap();
    let low = result.column("donchian_low_20").unwrap().f64().unwrap();
    let mid = result.column("donchian_mid_20").unwrap().f64().unwrap();

    for i in 20..result.height() {
        if let (Some(h), Some(l), Some(m)) = (high.get(i), low.get(i), mid.get(i)) {
            let expected = (h + l) / 2.0;
            assert!(
                (m - expected).abs() < 1e-10,
                "Mid should be average of high and low"
            );
        }
    }
}

#[test]
fn test_indicators_adx() {
    let lf = create_sample_lazyframe(60);
    let result = trendlab_data::indicators::add_adx(lf, 14).collect().unwrap();

    assert!(result.column("plus_di_14").is_ok());
    assert!(result.column("minus_di_14").is_ok());
    assert!(result.column("adx_14").is_ok());
}

#[test]
fn test_indicators_sma_ema() {
    let lf = create_sample_lazyframe(50);
    let lf = trendlab_data::indicators::add_sma(lf, 20);
    let result = trendlab_data::indicators::add_ema(lf, 20).collect().unwrap();

    assert!(result.column("sma_20").is_ok());
    assert!(result.column("ema_20").is_ok());
}

#[test]
fn test_indicators_chain() {
    // Test that multiple indicators can be chained without conflicts
    let lf = create_sample_lazyframe(100);

    let lf = trendlab_data::indicators::add_atr(lf, 14);
    let lf = trendlab_data::indicators::add_donchian(lf, 20);
    let lf = trendlab_data::indicators::add_adx(lf, 14);
    let lf = trendlab_data::indicators::add_sma(lf, 50);
    let lf = trendlab_data::indicators::add_returns(lf, 1);
    let result = trendlab_data::indicators::add_volatility(lf, 20).collect().unwrap();

    // All columns should exist
    assert!(result.column("atr_14").is_ok());
    assert!(result.column("donchian_high_20").is_ok());
    assert!(result.column("adx_14").is_ok());
    assert!(result.column("sma_50").is_ok());
    assert!(result.column("returns_1").is_ok());
    assert!(result.column("volatility_20").is_ok());
}

/// Test DataProvider creation and cache directory setup.
#[test]
fn test_data_provider_creation() {
    let dir = tempdir().unwrap();
    let provider = DataProvider::new(dir.path());
    assert!(provider.is_ok());
}

/// Test cache range when no data exists.
#[test]
fn test_cached_range_empty() {
    let dir = tempdir().unwrap();
    let provider = DataProvider::new(dir.path()).unwrap();
    let range = provider.cached_range("NONEXISTENT").unwrap();
    assert!(range.is_none());
}

/// Integration test with real Yahoo Finance API.
/// Marked as #[ignore] because it requires network access.
#[test]
#[ignore]
fn test_yahoo_fetch_live() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let dir = tempdir().unwrap();
        let provider = DataProvider::new(dir.path()).unwrap();

        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        // First fetch - should hit Yahoo
        let bars = provider.get_bars("AAPL", start, end).await.unwrap();
        assert!(!bars.is_empty());

        // Verify bars are sorted by date
        for window in bars.windows(2) {
            assert!(window[0].date <= window[1].date);
        }

        // Verify bar indices are sequential
        for (i, bar) in bars.iter().enumerate() {
            assert_eq!(bar.idx, i);
        }

        // Check cached range
        let range = provider.cached_range("AAPL").unwrap();
        assert!(range.is_some());
    });
}

/// Integration test for cache hit.
#[test]
#[ignore]
fn test_cache_hit() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let dir = tempdir().unwrap();
        let provider = DataProvider::new(dir.path()).unwrap();

        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        // First fetch
        let bars1 = provider.get_bars("MSFT", start, end).await.unwrap();

        // Second fetch - should use cache
        let bars2 = provider.get_bars("MSFT", start, end).await.unwrap();

        // Results should be identical
        assert_eq!(bars1.len(), bars2.len());
        for (b1, b2) in bars1.iter().zip(bars2.iter()) {
            assert_eq!(b1.date, b2.date);
            assert!((b1.close - b2.close).abs() < 1e-10);
        }
    });
}

/// Integration test with indicators.
#[test]
#[ignore]
fn test_data_with_indicators_live() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let dir = tempdir().unwrap();
        let provider = DataProvider::new(dir.path()).unwrap();

        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 3, 31).unwrap();

        let df = provider
            .get_data_with_indicators("AAPL", start, end)
            .await
            .unwrap()
            .collect()
            .unwrap();

        // Should have indicator columns
        assert!(df.column("atr_14").is_ok());
        assert!(df.column("donchian_high_20").is_ok());
        assert!(df.column("adx_14").is_ok());
    });
}

// Helper function to create sample OHLCV data
fn create_sample_lazyframe(n: usize) -> LazyFrame {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let dates: Vec<i32> = (0..n)
        .map(|i| {
            let date = base_date + chrono::Duration::days(i as i64);
            (date - epoch).num_days() as i32
        })
        .collect();

    // Generate trending synthetic data with some noise
    let base = 100.0;
    let opens: Vec<f64> = (0..n)
        .map(|i| base + (i as f64) * 0.5 + (i as f64 * 0.1).sin() * 2.0)
        .collect();

    let highs: Vec<f64> = opens.iter().map(|o| o + 3.0).collect();
    let lows: Vec<f64> = opens.iter().map(|o| o - 2.0).collect();
    let closes: Vec<f64> = opens.iter().map(|o| o + 1.0).collect();
    let volumes: Vec<u64> = (0..n).map(|i| 1_000_000 + (i as u64) * 1000).collect();

    DataFrame::new(vec![
        Column::new("date".into(), dates)
            .cast(&DataType::Date)
            .unwrap(),
        Column::new("open".into(), opens),
        Column::new("high".into(), highs),
        Column::new("low".into(), lows),
        Column::new("close".into(), closes),
        Column::new("volume".into(), volumes),
    ])
    .unwrap()
    .lazy()
}
