//! Breakeven Then Trail Position Manager.
//!
//! Moves stop to breakeven first, then begins trailing.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Breakeven Then Trail Position Manager.
///
/// # Strategy
///
/// Two-phase exit management:
/// 1. Initial phase: Fixed stop at `initial_stop_atr * ATR`
/// 2. After price moves `breakeven_threshold * ATR` in our favor:
///    - Move stop to breakeven (entry price)
///    - Begin trailing at `trail_atr * ATR`
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - tracks from entry forward.
///
/// # Parameters
///
/// - `initial_stop_atr`: Initial stop distance in ATR
/// - `breakeven_threshold`: Move required before breakeven (in ATR)
/// - `trail_atr`: Trailing stop distance after breakeven (in ATR)
#[derive(Debug, Clone)]
pub struct BreakevenThenTrail {
    initial_stop_atr: f64,
    breakeven_threshold: f64,
    trail_atr: f64,
    // Internal state
    stop_price: Option<f64>,
    entry_price: f64,
    at_breakeven: bool,
    high_since_entry: f64,
    low_since_entry: f64,
}

impl BreakevenThenTrail {
    /// Create a new breakeven-then-trail position manager.
    pub fn new(initial_stop_atr: f64, breakeven_threshold: f64, trail_atr: f64) -> Self {
        Self {
            initial_stop_atr,
            breakeven_threshold,
            trail_atr,
            stop_price: None,
            entry_price: 0.0,
            at_breakeven: false,
            high_since_entry: 0.0,
            low_since_entry: f64::MAX,
        }
    }
}

impl Default for BreakevenThenTrail {
    fn default() -> Self {
        Self::new(2.0, 1.5, 1.5) // 2x initial, 1.5x to BE, then 1.5x trail
    }
}

impl PositionManager for BreakevenThenTrail {
    fn name(&self) -> &str {
        "BreakevenThenTrail"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.entry_price = entry_price;
        self.at_breakeven = false;
        self.high_since_entry = entry_price;
        self.low_since_entry = entry_price;

        let bar_range = entry_bar.high - entry_bar.low;
        let initial_atr = if bar_range > 0.0 { bar_range } else { 1.0 };

        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price - (self.initial_stop_atr * initial_atr),
            Direction::Short => entry_price + (self.initial_stop_atr * initial_atr),
        });
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
        let atr = state.current_atr();
        if atr <= 0.0 {
            return Action::Hold;
        }

        // Update from engine-tracked position
        self.high_since_entry = position.high_since_entry;
        self.low_since_entry = position.low_since_entry;

        // Check if we should move to breakeven
        if !self.at_breakeven {
            let move_required = self.breakeven_threshold * atr;
            let favorable_move = match position.direction {
                Direction::Long => self.high_since_entry - self.entry_price,
                Direction::Short => self.entry_price - self.low_since_entry,
            };

            if favorable_move >= move_required {
                self.at_breakeven = true;
                self.stop_price = Some(self.entry_price);
                return Action::AdjustStop(self.entry_price);
            }
        }

        // If at breakeven, apply trailing logic
        if self.at_breakeven {
            let new_stop = match position.direction {
                Direction::Long => self.high_since_entry - (self.trail_atr * atr),
                Direction::Short => self.low_since_entry + (self.trail_atr * atr),
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
        vec![
            ParamDef {
                name: "initial_stop_atr".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 4.0,
                    step: 0.5,
                },
                description: Some("Initial stop distance in ATR".into()),
            },
            ParamDef {
                name: "breakeven_threshold".into(),
                param_type: ParamType::Float {
                    min: 0.5,
                    max: 3.0,
                    step: 0.5,
                },
                description: Some("Move required for breakeven (ATR)".into()),
            },
            ParamDef {
                name: "trail_atr".into(),
                param_type: ParamType::Float {
                    min: 0.5,
                    max: 3.0,
                    step: 0.5,
                },
                description: Some("Trail distance after breakeven (ATR)".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(
            self.initial_stop_atr,
            self.breakeven_threshold,
            self.trail_atr,
        ))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.entry_price = 0.0;
        self.at_breakeven = false;
        self.high_since_entry = 0.0;
        self.low_since_entry = f64::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_reference_mode() {
        let pm = BreakevenThenTrail::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }

    #[test]
    fn test_name() {
        let pm = BreakevenThenTrail::default();
        assert_eq!(pm.name(), "BreakevenThenTrail");
    }
}
