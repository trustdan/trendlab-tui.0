//! Fill result types for execution.

use serde::{Deserialize, Serialize};

/// Result of an order execution attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillResult {
    /// Whether the order was filled
    pub filled: bool,

    /// Price at which the order was filled (if filled)
    pub fill_price: f64,

    /// Bar index when fill occurred
    pub fill_bar_idx: usize,

    /// Slippage incurred (signed: positive = adverse)
    pub slippage: f64,

    /// Commission charged
    pub commission: f64,
}

impl FillResult {
    /// Create a successful fill result.
    pub fn filled(price: f64, bar_idx: usize, slippage: f64, commission: f64) -> Self {
        Self {
            filled: true,
            fill_price: price,
            fill_bar_idx: bar_idx,
            slippage,
            commission,
        }
    }

    /// Create a no-fill result.
    pub fn not_filled(bar_idx: usize) -> Self {
        Self {
            filled: false,
            fill_price: 0.0,
            fill_bar_idx: bar_idx,
            slippage: 0.0,
            commission: 0.0,
        }
    }

    /// Total execution cost (slippage + commission).
    #[inline]
    pub fn total_cost(&self) -> f64 {
        self.slippage + self.commission
    }
}

/// Policy for handling gaps through stop prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GapPolicy {
    /// Fill at open price if gap through stop (realistic)
    #[default]
    FillAtOpen,
    /// Fill at stop price (unrealistic but sometimes used)
    FillAtStop,
    /// No fill on gap (position remains open)
    NoFill,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_result() {
        let fill = FillResult::filled(100.0, 5, 0.05, 1.0);
        assert!(fill.filled);
        assert_eq!(fill.fill_price, 100.0);
        assert_eq!(fill.total_cost(), 1.05);
    }

    #[test]
    fn test_not_filled() {
        let no_fill = FillResult::not_filled(5);
        assert!(!no_fill.filled);
        assert_eq!(no_fill.total_cost(), 0.0);
    }
}
