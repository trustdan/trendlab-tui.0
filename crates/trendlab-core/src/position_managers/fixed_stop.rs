//! Fixed Stop Position Manager.
//!
//! Simple fixed stop loss and optional take profit from entry price.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Fixed Stop Position Manager.
///
/// # Strategy
///
/// Fixed stop loss and take profit levels based on entry price:
/// - Stop loss at `entry_price * (1 - stop_percent)` for longs
/// - Take profit at `entry_price * (1 + target_percent)` for longs
/// - Reversed for shorts
///
/// # Exit Reference Mode
///
/// Uses `EntryFrozenReference` - stop is fixed at entry, never moves.
///
/// # Parameters
///
/// - `stop_percent`: Stop loss distance as percentage (e.g., 0.02 = 2%)
/// - `target_percent`: Take profit distance as percentage (0 = no target)
#[derive(Debug, Clone)]
pub struct FixedStop {
    stop_percent: f64,
    target_percent: f64,
    // Internal state
    stop_price: Option<f64>,
    target_price: Option<f64>,
    direction: Option<Direction>,
}

impl FixedStop {
    /// Create a new fixed stop position manager.
    pub fn new(stop_percent: f64, target_percent: f64) -> Self {
        Self {
            stop_percent,
            target_percent,
            stop_price: None,
            target_price: None,
            direction: None,
        }
    }
}

impl Default for FixedStop {
    fn default() -> Self {
        Self::new(0.02, 0.0) // 2% stop, no target
    }
}

impl PositionManager for FixedStop {
    fn name(&self) -> &str {
        "FixedStop"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::EntryFrozenReference)
    }

    fn on_entry(&mut self, _entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.direction = Some(signal.direction);

        match signal.direction {
            Direction::Long => {
                self.stop_price = Some(entry_price * (1.0 - self.stop_percent));
                self.target_price = if self.target_percent > 0.0 {
                    Some(entry_price * (1.0 + self.target_percent))
                } else {
                    None
                };
            }
            Direction::Short => {
                self.stop_price = Some(entry_price * (1.0 + self.stop_percent));
                self.target_price = if self.target_percent > 0.0 {
                    Some(entry_price * (1.0 - self.target_percent))
                } else {
                    None
                };
            }
        }
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, _state: &MarketState) -> Action {
        let direction = self.direction.unwrap_or(position.direction);

        // Check target first (if set)
        if let Some(target) = self.target_price {
            let target_hit = match direction {
                Direction::Long => bar.high >= target,
                Direction::Short => bar.low <= target,
            };
            if target_hit {
                return Action::Exit(ExitReason::TakeProfit);
            }
        }

        // Check stop
        if let Some(stop) = self.stop_price {
            let stop_hit = match direction {
                Direction::Long => bar.low <= stop,
                Direction::Short => bar.high >= stop,
            };
            if stop_hit {
                return Action::Exit(ExitReason::StopHit);
            }
        }

        Action::Hold
    }

    fn stop_price(&self) -> Option<f64> {
        self.stop_price
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "stop_percent".into(),
                param_type: ParamType::Float {
                    min: 0.005,
                    max: 0.10,
                    step: 0.005,
                },
                description: Some("Stop loss percentage".into()),
            },
            ParamDef {
                name: "target_percent".into(),
                param_type: ParamType::Float {
                    min: 0.0,
                    max: 0.20,
                    step: 0.02,
                },
                description: Some("Take profit percentage (0 = none)".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.stop_percent, self.target_percent))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.target_price = None;
        self.direction = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_reference_mode() {
        let pm = FixedStop::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::EntryFrozenReference)
        );
    }

    #[test]
    fn test_name() {
        let pm = FixedStop::default();
        assert_eq!(pm.name(), "FixedStop");
    }
}
