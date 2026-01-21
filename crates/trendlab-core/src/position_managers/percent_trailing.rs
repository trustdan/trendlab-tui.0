//! Percent Trailing Stop Position Manager.
//!
//! Simple percentage-based trailing stop from the highest/lowest since entry.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Percent Trailing Stop Position Manager.
///
/// # Strategy
///
/// A simple percentage-based trailing stop:
/// - Long stop: `high_since_entry * (1 - trail_percent)`
/// - Short stop: `low_since_entry * (1 + trail_percent)`
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - extremes are tracked from entry forward.
///
/// # Parameters
///
/// - `trail_percent`: Percentage distance from extreme (e.g., 0.05 = 5%)
#[derive(Debug, Clone)]
pub struct PercentTrailing {
    trail_percent: f64,
    // Internal state
    stop_price: Option<f64>,
    high_since_entry: f64,
    low_since_entry: f64,
}

impl PercentTrailing {
    /// Create a new percent trailing stop position manager.
    pub fn new(trail_percent: f64) -> Self {
        Self {
            trail_percent,
            stop_price: None,
            high_since_entry: 0.0,
            low_since_entry: f64::MAX,
        }
    }
}

impl Default for PercentTrailing {
    fn default() -> Self {
        Self::new(0.05) // 5% trailing stop
    }
}

impl PositionManager for PercentTrailing {
    fn name(&self) -> &str {
        "PercentTrailing"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, _entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.high_since_entry = entry_price;
        self.low_since_entry = entry_price;

        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price * (1.0 - self.trail_percent),
            Direction::Short => entry_price * (1.0 + self.trail_percent),
        });
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, _state: &MarketState) -> Action {
        // Update from engine-tracked position
        self.high_since_entry = position.high_since_entry;
        self.low_since_entry = position.low_since_entry;

        // Calculate new trailing stop
        let new_stop = match position.direction {
            Direction::Long => self.high_since_entry * (1.0 - self.trail_percent),
            Direction::Short => self.low_since_entry * (1.0 + self.trail_percent),
        };

        // Only ratchet stop in favorable direction
        let should_update = match position.direction {
            Direction::Long => self.stop_price.map(|s| new_stop > s).unwrap_or(true),
            Direction::Short => self.stop_price.map(|s| new_stop < s).unwrap_or(true),
        };

        if should_update {
            self.stop_price = Some(new_stop);
            return Action::AdjustStop(new_stop);
        }

        // Check if stop is hit
        let stop = self.stop_price.unwrap_or(0.0);
        let stop_hit = match position.direction {
            Direction::Long => bar.low <= stop,
            Direction::Short => bar.high >= stop,
        };

        if stop_hit {
            return Action::Exit(ExitReason::StopHit);
        }

        Action::Hold
    }

    fn stop_price(&self) -> Option<f64> {
        self.stop_price
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![ParamDef {
            name: "trail_percent".into(),
            param_type: ParamType::Float {
                min: 0.01,
                max: 0.20,
                step: 0.01,
            },
            description: Some("Trailing stop percentage".into()),
        }]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.trail_percent))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.high_since_entry = 0.0;
        self.low_since_entry = f64::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let pm = PercentTrailing::default();
        assert_eq!(pm.trail_percent, 0.05);
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = PercentTrailing::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }
}
