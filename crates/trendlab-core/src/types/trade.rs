//! Completed trade record.

use super::{Direction, ExitReason};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Completed trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Bar index when position was opened
    pub entry_bar_idx: usize,
    /// Date of entry
    pub entry_date: NaiveDate,
    /// Fill price at entry
    pub entry_price: f64,

    /// Bar index when position was closed
    pub exit_bar_idx: usize,
    /// Date of exit
    pub exit_date: NaiveDate,
    /// Fill price at exit
    pub exit_price: f64,

    /// Trade direction
    pub direction: Direction,
    /// Position size in dollars
    pub size: f64,
    /// Why the trade was closed
    pub exit_reason: ExitReason,

    /// Return as decimal (0.05 = 5%)
    pub return_pct: f64,

    /// Bars held
    pub bars_held: usize,

    /// Maximum Adverse Excursion (worst drawdown during trade)
    pub mae: f64,

    /// Maximum Favorable Excursion (best gain during trade)
    pub mfe: f64,
}

impl Trade {
    /// Create a trade record from a closed position.
    pub fn new(
        entry_bar_idx: usize,
        entry_date: NaiveDate,
        entry_price: f64,
        exit_bar_idx: usize,
        exit_date: NaiveDate,
        exit_price: f64,
        direction: Direction,
        size: f64,
        exit_reason: ExitReason,
        high_since_entry: f64,
        low_since_entry: f64,
        bars_held: usize,
    ) -> Self {
        let return_pct = match direction {
            Direction::Long => (exit_price - entry_price) / entry_price,
            Direction::Short => (entry_price - exit_price) / entry_price,
        };

        let (mae, mfe) = match direction {
            Direction::Long => (
                (low_since_entry - entry_price) / entry_price,
                (high_since_entry - entry_price) / entry_price,
            ),
            Direction::Short => (
                (entry_price - high_since_entry) / entry_price,
                (entry_price - low_since_entry) / entry_price,
            ),
        };

        Self {
            entry_bar_idx,
            entry_date,
            entry_price,
            exit_bar_idx,
            exit_date,
            exit_price,
            direction,
            size,
            exit_reason,
            return_pct,
            bars_held,
            mae,
            mfe,
        }
    }

    /// Returns true if this trade was profitable.
    #[inline]
    pub fn is_winner(&self) -> bool {
        self.return_pct > 0.0
    }

    /// Returns true if this trade was a loss.
    #[inline]
    pub fn is_loser(&self) -> bool {
        self.return_pct < 0.0
    }

    /// Dollar profit/loss for this trade.
    #[inline]
    pub fn pnl_dollars(&self) -> f64 {
        self.return_pct * self.size
    }

    /// Trade duration in bars.
    #[inline]
    pub fn duration(&self) -> usize {
        self.exit_bar_idx.saturating_sub(self.entry_bar_idx)
    }

    /// Efficiency: how much of the favorable excursion was captured.
    /// 1.0 = exited at the best price, 0.0 = gave back all gains.
    pub fn efficiency(&self) -> f64 {
        if self.mfe.abs() < 1e-10 {
            0.0
        } else {
            (self.return_pct / self.mfe).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trade() -> Trade {
        Trade::new(
            10,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            100.0,
            20,
            NaiveDate::from_ymd_opt(2024, 1, 29).unwrap(),
            110.0,
            Direction::Long,
            10000.0,
            ExitReason::TakeProfit,
            115.0, // high_since_entry
            95.0,  // low_since_entry
            10,
        )
    }

    #[test]
    fn test_trade_return() {
        let trade = sample_trade();
        assert!((trade.return_pct - 0.10).abs() < 1e-10); // 10% return
    }

    #[test]
    fn test_trade_mae_mfe() {
        let trade = sample_trade();
        assert!((trade.mae - (-0.05)).abs() < 1e-10); // 5% drawdown
        assert!((trade.mfe - 0.15).abs() < 1e-10); // 15% max gain
    }

    #[test]
    fn test_is_winner() {
        let trade = sample_trade();
        assert!(trade.is_winner());
        assert!(!trade.is_loser());
    }

    #[test]
    fn test_duration() {
        let trade = sample_trade();
        assert_eq!(trade.duration(), 10);
    }

    #[test]
    fn test_efficiency() {
        let trade = sample_trade();
        // Captured 10% of 15% MFE = 66.7% efficiency
        assert!((trade.efficiency() - (0.10 / 0.15)).abs() < 1e-10);
    }
}
