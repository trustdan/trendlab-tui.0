//! Open position tracking.

use super::{Bar, Direction, Signal};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Open position during backtest.
///
/// # Key Invariant
///
/// `high_since_entry` and `low_since_entry` are tracked FROM ENTRY FORWARD,
/// not from historical data. This prevents the stickiness problem where
/// exit references "run away" with global extremes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Bar index when position was opened
    pub entry_bar_idx: usize,

    /// Date of entry
    pub entry_date: NaiveDate,

    /// Fill price at entry
    pub entry_price: f64,

    /// Trade direction
    pub direction: Direction,

    /// Position size in dollars
    pub size: f64,

    /// Signal that triggered entry
    pub signal: Signal,

    /// Highest price SINCE ENTRY (not global!)
    pub high_since_entry: f64,

    /// Lowest price SINCE ENTRY (not global!)
    pub low_since_entry: f64,

    /// Number of bars held
    pub bars_held: usize,

    /// Current stop price (if any)
    pub stop_price: Option<f64>,
}

impl Position {
    /// Create a new position at entry.
    ///
    /// # Important
    /// Extremes are initialized to entry price, NOT historical values.
    pub fn new(
        entry_bar_idx: usize,
        entry_date: NaiveDate,
        entry_price: f64,
        direction: Direction,
        size: f64,
        signal: Signal,
    ) -> Self {
        Self {
            entry_bar_idx,
            entry_date,
            entry_price,
            direction,
            size,
            signal,
            high_since_entry: entry_price,
            low_since_entry: entry_price,
            bars_held: 0,
            stop_price: None,
        }
    }

    /// Update tracking for a new bar.
    ///
    /// This MUST be called at the start of each bar processing.
    pub fn update_for_bar(&mut self, bar: &Bar) {
        self.high_since_entry = self.high_since_entry.max(bar.high);
        self.low_since_entry = self.low_since_entry.min(bar.low);
        self.bars_held += 1;
    }

    /// Calculate unrealized P&L percentage.
    pub fn unrealized_pnl_pct(&self, current_price: f64) -> f64 {
        match self.direction {
            Direction::Long => (current_price - self.entry_price) / self.entry_price,
            Direction::Short => (self.entry_price - current_price) / self.entry_price,
        }
    }

    /// Calculate unrealized P&L in dollars.
    pub fn unrealized_pnl_dollars(&self, current_price: f64) -> f64 {
        self.unrealized_pnl_pct(current_price) * self.size
    }

    /// Maximum Adverse Excursion (worst drawdown during trade) as percentage.
    pub fn mae(&self) -> f64 {
        match self.direction {
            Direction::Long => (self.low_since_entry - self.entry_price) / self.entry_price,
            Direction::Short => (self.entry_price - self.high_since_entry) / self.entry_price,
        }
    }

    /// Maximum Favorable Excursion (best gain during trade) as percentage.
    pub fn mfe(&self) -> f64 {
        match self.direction {
            Direction::Long => (self.high_since_entry - self.entry_price) / self.entry_price,
            Direction::Short => (self.entry_price - self.low_since_entry) / self.entry_price,
        }
    }

    /// Set the stop price.
    pub fn set_stop(&mut self, stop: f64) {
        self.stop_price = Some(stop);
    }

    /// Check if stop would be hit at given price.
    pub fn is_stop_hit(&self, low: f64, high: f64) -> bool {
        match (self.stop_price, self.direction) {
            (Some(stop), Direction::Long) => low <= stop,
            (Some(stop), Direction::Short) => high >= stop,
            (None, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_position() -> Position {
        Position::new(
            10,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            100.0,
            Direction::Long,
            10000.0,
            Signal::market(Direction::Long, 1.0, 100.0),
        )
    }

    #[test]
    fn test_new_position_extremes() {
        let pos = sample_position();
        // Key invariant: extremes start at entry price
        assert_eq!(pos.high_since_entry, 100.0);
        assert_eq!(pos.low_since_entry, 100.0);
    }

    #[test]
    fn test_update_for_bar() {
        let mut pos = sample_position();

        let bar = Bar::new(
            NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(),
            101.0,
            110.0,
            99.0,
            105.0,
            1_000_000,
            11,
        );

        pos.update_for_bar(&bar);

        assert_eq!(pos.high_since_entry, 110.0);
        assert_eq!(pos.low_since_entry, 99.0);
        assert_eq!(pos.bars_held, 1);
    }

    #[test]
    fn test_unrealized_pnl() {
        let pos = sample_position();
        // 5% gain
        assert!((pos.unrealized_pnl_pct(105.0) - 0.05).abs() < 1e-10);
        // 5% loss
        assert!((pos.unrealized_pnl_pct(95.0) - (-0.05)).abs() < 1e-10);
    }

    #[test]
    fn test_stop_hit() {
        let mut pos = sample_position();
        pos.set_stop(95.0);

        // Stop not hit
        assert!(!pos.is_stop_hit(96.0, 105.0));
        // Stop hit
        assert!(pos.is_stop_hit(94.0, 105.0));
    }
}
