//! Read-only market context for components.
//!
//! # Lookahead Prevention
//!
//! This struct provides a controlled view of market data. Components receive
//! only bars up to and including the current bar index, preventing lookahead bias.

use crate::types::Bar;

/// Read-only market context available to components.
///
/// # Contract
/// - `bars.len() == current_idx + 1`
/// - All indicator arrays have same length as bars
///
/// # Lookahead Prevention
///
/// Components receive this struct which contains only bars up to and including
/// the current bar. There is NO way to access future bars through this interface.
#[derive(Debug, Clone)]
pub struct MarketState<'a> {
    /// Bars from index 0 to current_idx (inclusive)
    pub bars: &'a [Bar],

    /// Current bar index
    pub current_idx: usize,

    /// Pre-computed ATR (14-period) at each bar
    pub atr: &'a [f64],

    /// Pre-computed ADX (14-period) at each bar
    pub adx: &'a [f64],
}

impl<'a> MarketState<'a> {
    /// Create a new market state.
    ///
    /// # Panics
    /// Panics if bars is empty or if indicator arrays don't match bars length.
    pub fn new(bars: &'a [Bar], current_idx: usize, atr: &'a [f64], adx: &'a [f64]) -> Self {
        debug_assert!(!bars.is_empty(), "bars cannot be empty");
        debug_assert!(current_idx < bars.len(), "current_idx out of bounds");
        debug_assert_eq!(bars.len(), atr.len(), "ATR length must match bars");
        debug_assert_eq!(bars.len(), adx.len(), "ADX length must match bars");

        Self {
            bars,
            current_idx,
            atr,
            adx,
        }
    }

    /// Get the current bar.
    #[inline]
    pub fn current_bar(&self) -> &Bar {
        &self.bars[self.current_idx]
    }

    /// Get bars from N periods ago to current (inclusive).
    ///
    /// If N is larger than available history, returns from the start.
    pub fn lookback(&self, periods: usize) -> &[Bar] {
        let start = self.current_idx.saturating_sub(periods);
        &self.bars[start..=self.current_idx]
    }

    /// Highest high over last N bars (excluding current).
    ///
    /// Returns `f64::MIN` if no historical bars available.
    pub fn highest_high(&self, periods: usize) -> f64 {
        if self.current_idx == 0 {
            return f64::MIN;
        }

        let start = self.current_idx.saturating_sub(periods);
        self.bars[start..self.current_idx]
            .iter()
            .map(|b| b.high)
            .fold(f64::MIN, f64::max)
    }

    /// Lowest low over last N bars (excluding current).
    ///
    /// Returns `f64::MAX` if no historical bars available.
    pub fn lowest_low(&self, periods: usize) -> f64 {
        if self.current_idx == 0 {
            return f64::MAX;
        }

        let start = self.current_idx.saturating_sub(periods);
        self.bars[start..self.current_idx]
            .iter()
            .map(|b| b.low)
            .fold(f64::MAX, f64::min)
    }

    /// Highest high over last N bars (including current).
    pub fn highest_high_inclusive(&self, periods: usize) -> f64 {
        let start = self.current_idx.saturating_sub(periods.saturating_sub(1));
        self.bars[start..=self.current_idx]
            .iter()
            .map(|b| b.high)
            .fold(f64::MIN, f64::max)
    }

    /// Lowest low over last N bars (including current).
    pub fn lowest_low_inclusive(&self, periods: usize) -> f64 {
        let start = self.current_idx.saturating_sub(periods.saturating_sub(1));
        self.bars[start..=self.current_idx]
            .iter()
            .map(|b| b.low)
            .fold(f64::MAX, f64::min)
    }

    /// Get the previous bar, if available.
    pub fn prev_bar(&self) -> Option<&Bar> {
        if self.current_idx > 0 {
            Some(&self.bars[self.current_idx - 1])
        } else {
            None
        }
    }

    /// Get a bar N periods ago, if available.
    pub fn bar_ago(&self, n: usize) -> Option<&Bar> {
        if n <= self.current_idx {
            Some(&self.bars[self.current_idx - n])
        } else {
            None
        }
    }

    /// Current ATR value.
    #[inline]
    pub fn current_atr(&self) -> f64 {
        self.atr.get(self.current_idx).copied().unwrap_or(0.0)
    }

    /// Current ADX value.
    #[inline]
    pub fn current_adx(&self) -> f64 {
        self.adx.get(self.current_idx).copied().unwrap_or(0.0)
    }

    /// ATR N bars ago.
    pub fn atr_ago(&self, n: usize) -> Option<f64> {
        if n <= self.current_idx {
            self.atr.get(self.current_idx - n).copied()
        } else {
            None
        }
    }

    /// Number of bars available (including current).
    #[inline]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Returns true if there are no bars.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// Simple moving average of closes over last N bars.
    pub fn sma_close(&self, periods: usize) -> f64 {
        let bars = self.lookback(periods.saturating_sub(1));
        if bars.is_empty() {
            return 0.0;
        }
        bars.iter().map(|b| b.close).sum::<f64>() / bars.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0 + i as f64,
                high: 105.0 + i as f64,
                low: 95.0 + i as f64,
                close: 102.0 + i as f64,
                volume: 1_000_000,
                idx: i,
            })
            .collect()
    }

    #[test]
    fn test_current_bar() {
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars, 5, &atr, &adx);

        assert_eq!(state.current_bar().idx, 5);
    }

    #[test]
    fn test_highest_high_excludes_current() {
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars, 5, &atr, &adx);

        // Looking back 3 bars from index 5: bars 2, 3, 4
        // Highs: 107, 108, 109 (current bar 5 has high 110, excluded)
        let hh = state.highest_high(3);
        assert_eq!(hh, 109.0);
    }

    #[test]
    fn test_lowest_low_excludes_current() {
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars, 5, &atr, &adx);

        // Looking back 3 bars: bars 2, 3, 4
        // Lows: 97, 98, 99 (current bar 5 has low 100, excluded)
        let ll = state.lowest_low(3);
        assert_eq!(ll, 97.0);
    }

    #[test]
    fn test_no_lookahead() {
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars[..6], 5, &atr[..6], &adx[..6]);

        // State only has bars 0-5, cannot access bar 6+
        assert_eq!(state.len(), 6);
        assert_eq!(state.current_idx, 5);
    }

    #[test]
    fn test_sma_close() {
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars, 5, &atr, &adx);

        // SMA of last 3 closes: bars 3, 4, 5 have closes 105, 106, 107 = 318 / 3 = 106
        let sma = state.sma_close(3);
        assert!((sma - 106.0).abs() < 1e-10);
    }
}
