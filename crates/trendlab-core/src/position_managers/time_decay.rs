//! Time Decay Stop Position Manager.
//!
//! Stop tightens progressively over the holding period.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Time Decay Stop Position Manager.
///
/// # Strategy
///
/// The stop distance tightens linearly over time:
/// - Initial stop at `initial_atr_mult * ATR`
/// - Final stop at `final_atr_mult * ATR` after `decay_bars`
///
/// This encourages taking profits as the trade ages.
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - tracks from entry forward with decay.
///
/// # Parameters
///
/// - `initial_atr_mult`: Initial ATR multiplier for stop
/// - `final_atr_mult`: Final ATR multiplier after decay period
/// - `decay_bars`: Number of bars for full decay
#[derive(Debug, Clone)]
pub struct TimeDecayStop {
    initial_atr_mult: f64,
    final_atr_mult: f64,
    decay_bars: usize,
    // Internal state
    stop_price: Option<f64>,
    entry_bar_idx: usize,
    high_since_entry: f64,
    low_since_entry: f64,
}

impl TimeDecayStop {
    /// Create a new time decay stop position manager.
    pub fn new(initial_atr_mult: f64, final_atr_mult: f64, decay_bars: usize) -> Self {
        Self {
            initial_atr_mult,
            final_atr_mult,
            decay_bars,
            stop_price: None,
            entry_bar_idx: 0,
            high_since_entry: 0.0,
            low_since_entry: f64::MAX,
        }
    }

    /// Calculate current ATR multiplier based on bars held.
    fn current_multiplier(&self, bars_held: usize) -> f64 {
        if bars_held >= self.decay_bars {
            return self.final_atr_mult;
        }

        let progress = bars_held as f64 / self.decay_bars as f64;
        self.initial_atr_mult + (self.final_atr_mult - self.initial_atr_mult) * progress
    }
}

impl Default for TimeDecayStop {
    fn default() -> Self {
        Self::new(3.0, 1.0, 20) // 3x ATR -> 1x ATR over 20 bars
    }
}

impl PositionManager for TimeDecayStop {
    fn name(&self) -> &str {
        "TimeDecayStop"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.entry_bar_idx = entry_bar.idx;
        self.high_since_entry = entry_price;
        self.low_since_entry = entry_price;

        let bar_range = entry_bar.high - entry_bar.low;
        let initial_atr = if bar_range > 0.0 { bar_range } else { 1.0 };

        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price - (self.initial_atr_mult * initial_atr),
            Direction::Short => entry_price + (self.initial_atr_mult * initial_atr),
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

        // Calculate time-decayed multiplier
        let bars_held = bar.idx.saturating_sub(self.entry_bar_idx);
        let multiplier = self.current_multiplier(bars_held);

        // Calculate new trailing stop
        let new_stop = match position.direction {
            Direction::Long => self.high_since_entry - (multiplier * atr),
            Direction::Short => self.low_since_entry + (multiplier * atr),
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
        vec![
            ParamDef {
                name: "initial_atr_mult".into(),
                param_type: ParamType::Float {
                    min: 2.0,
                    max: 5.0,
                    step: 0.5,
                },
                description: Some("Initial ATR multiplier".into()),
            },
            ParamDef {
                name: "final_atr_mult".into(),
                param_type: ParamType::Float {
                    min: 0.5,
                    max: 2.0,
                    step: 0.25,
                },
                description: Some("Final ATR multiplier".into()),
            },
            ParamDef {
                name: "decay_bars".into(),
                param_type: ParamType::Int {
                    min: 5,
                    max: 60,
                    step: 5,
                },
                description: Some("Bars for full decay".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(
            self.initial_atr_mult,
            self.final_atr_mult,
            self.decay_bars,
        ))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.entry_bar_idx = 0;
        self.high_since_entry = 0.0;
        self.low_since_entry = f64::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiplier_decay() {
        let pm = TimeDecayStop::new(4.0, 1.0, 30);

        assert!((pm.current_multiplier(0) - 4.0).abs() < 0.01);
        assert!((pm.current_multiplier(15) - 2.5).abs() < 0.01); // Halfway
        assert!((pm.current_multiplier(30) - 1.0).abs() < 0.01);
        assert!((pm.current_multiplier(100) - 1.0).abs() < 0.01); // Capped at final
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = TimeDecayStop::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }
}
