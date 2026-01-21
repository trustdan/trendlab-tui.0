//! Entry signal type.

use super::Direction;
use serde::{Deserialize, Serialize};

/// Entry signal produced by a SignalGenerator.
///
/// # Contract
/// - Signals represent entry *intent*, not orders
/// - Signals MUST NOT contain exit information
/// - Signal strength should be normalized [0.0, 1.0] where practical
///   (generators should clamp if their raw strength can exceed 1.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Direction of the proposed trade
    pub direction: Direction,

    /// Optional limit/stop price for entry (None = market order)
    pub entry_level: Option<f64>,

    /// Signal strength or confidence [0.0, 1.0]
    pub strength: f64,

    /// The indicator value that triggered the signal (for debugging/export)
    pub trigger_value: f64,
}

impl Signal {
    /// Create a new market entry signal.
    pub fn market(direction: Direction, strength: f64, trigger_value: f64) -> Self {
        Self {
            direction,
            entry_level: None,
            strength: strength.clamp(0.0, 1.0),
            trigger_value,
        }
    }

    /// Create a new limit entry signal.
    pub fn limit(direction: Direction, level: f64, strength: f64, trigger_value: f64) -> Self {
        Self {
            direction,
            entry_level: Some(level),
            strength: strength.clamp(0.0, 1.0),
            trigger_value,
        }
    }

    /// Create a new stop entry signal (buy stop or sell stop).
    pub fn stop(direction: Direction, level: f64, strength: f64, trigger_value: f64) -> Self {
        Self {
            direction,
            entry_level: Some(level),
            strength: strength.clamp(0.0, 1.0),
            trigger_value,
        }
    }

    /// Returns true if this is a market order signal.
    #[inline]
    pub fn is_market(&self) -> bool {
        self.entry_level.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_signal() {
        let sig = Signal::market(Direction::Long, 0.8, 150.0);
        assert!(sig.is_market());
        assert_eq!(sig.direction, Direction::Long);
        assert_eq!(sig.strength, 0.8);
    }

    #[test]
    fn test_strength_clamping() {
        let sig = Signal::market(Direction::Long, 1.5, 150.0);
        assert_eq!(sig.strength, 1.0);

        let sig2 = Signal::market(Direction::Short, -0.5, 150.0);
        assert_eq!(sig2.strength, 0.0);
    }

    #[test]
    fn test_limit_signal() {
        let sig = Signal::limit(Direction::Long, 100.0, 0.9, 105.0);
        assert!(!sig.is_market());
        assert_eq!(sig.entry_level, Some(100.0));
    }
}
