//! OHLCV bar type.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A single OHLCV bar representing one trading day.
///
/// # Invariants
/// - `high >= low`
/// - `high >= open` and `high >= close`
/// - `low <= open` and `low <= close`
/// - `volume >= 0`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    /// Trading date (no time component for daily bars)
    pub date: NaiveDate,

    /// Opening price
    pub open: f64,

    /// Highest price during the period
    pub high: f64,

    /// Lowest price during the period
    pub low: f64,

    /// Closing price
    pub close: f64,

    /// Trading volume
    pub volume: u64,

    /// Zero-based index in the bar sequence
    pub idx: usize,
}

impl Bar {
    /// Create a new bar with validation.
    ///
    /// # Panics
    /// Panics if high < low or if high/low don't contain open/close.
    pub fn new(
        date: NaiveDate,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: u64,
        idx: usize,
    ) -> Self {
        debug_assert!(high >= low, "high must be >= low");
        debug_assert!(high >= open && high >= close, "high must be >= open and close");
        debug_assert!(low <= open && low <= close, "low must be <= open and close");

        Self {
            date,
            open,
            high,
            low,
            close,
            volume,
            idx,
        }
    }

    /// True range for this bar (requires previous close for accuracy).
    ///
    /// True Range = max(high - low, |high - prev_close|, |low - prev_close|)
    pub fn true_range(&self, prev_close: Option<f64>) -> f64 {
        let hl = self.high - self.low;
        match prev_close {
            Some(pc) => {
                let hc = (self.high - pc).abs();
                let lc = (self.low - pc).abs();
                hl.max(hc).max(lc)
            }
            None => hl,
        }
    }

    /// Typical price: (high + low + close) / 3
    #[inline]
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// VWAP approximation using typical price.
    #[inline]
    pub fn vwap_approx(&self) -> f64 {
        self.typical_price()
    }

    /// Bar range (high - low).
    #[inline]
    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    /// Body size (absolute difference between open and close).
    #[inline]
    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Returns true if this is a bullish bar (close > open).
    #[inline]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Returns true if this is a bearish bar (close < open).
    #[inline]
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bar() -> Bar {
        Bar::new(
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            100.0,
            105.0,
            98.0,
            103.0,
            1_000_000,
            0,
        )
    }

    #[test]
    fn test_true_range_no_prev() {
        let bar = sample_bar();
        assert_eq!(bar.true_range(None), 7.0); // 105 - 98
    }

    #[test]
    fn test_true_range_with_prev() {
        let bar = sample_bar();
        // prev_close = 95, so |high - prev| = 10, |low - prev| = 3, hl = 7
        assert_eq!(bar.true_range(Some(95.0)), 10.0);
    }

    #[test]
    fn test_typical_price() {
        let bar = sample_bar();
        let expected = (105.0 + 98.0 + 103.0) / 3.0;
        assert!((bar.typical_price() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_bullish_bearish() {
        let bar = sample_bar();
        assert!(bar.is_bullish()); // 103 > 100

        let bearish = Bar::new(
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            100.0,
            102.0,
            95.0,
            97.0,
            1_000_000,
            0,
        );
        assert!(bearish.is_bearish());
    }
}
