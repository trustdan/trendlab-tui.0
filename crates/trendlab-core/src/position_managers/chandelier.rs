//! Chandelier Exit Position Manager.
//!
//! A volatility-based trailing stop that trails from the highest high (for longs)
//! or lowest low (for shorts) of the lookback period since entry.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Chandelier Exit Position Manager.
///
/// # Strategy
///
/// The Chandelier Exit hangs from the highest high (longs) or lowest low (shorts):
/// - Long stop: `highest_high_N - (multiplier * ATR)`
/// - Short stop: `lowest_low_N + (multiplier * ATR)`
///
/// Unlike ATR trailing which tracks from entry only, Chandelier uses a rolling
/// lookback within the trade (but still starting from entry).
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - extremes are tracked from entry forward.
///
/// # Parameters
///
/// - `lookback`: Number of bars to look back for highest high/lowest low
/// - `atr_multiplier`: Distance from extreme to stop in ATR units
#[derive(Debug, Clone)]
pub struct ChandelierExit {
    lookback: usize,
    atr_multiplier: f64,
    // Internal state
    stop_price: Option<f64>,
    entry_bar_idx: usize,
    direction: Option<Direction>,
}

impl ChandelierExit {
    /// Create a new Chandelier exit position manager.
    pub fn new(lookback: usize, atr_multiplier: f64) -> Self {
        Self {
            lookback,
            atr_multiplier,
            stop_price: None,
            entry_bar_idx: 0,
            direction: None,
        }
    }
}

impl Default for ChandelierExit {
    fn default() -> Self {
        Self::new(22, 3.0)
    }
}

impl PositionManager for ChandelierExit {
    fn name(&self) -> &str {
        "ChandelierExit"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.entry_bar_idx = entry_bar.idx;
        self.direction = Some(signal.direction);

        let bar_range = entry_bar.high - entry_bar.low;
        let initial_atr = if bar_range > 0.0 { bar_range } else { 1.0 };

        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price - (self.atr_multiplier * initial_atr),
            Direction::Short => entry_price + (self.atr_multiplier * initial_atr),
        });
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
        let atr = state.current_atr();
        if atr <= 0.0 {
            return Action::Hold;
        }

        let direction = self.direction.unwrap_or(position.direction);

        // Calculate lookback window (capped to bars since entry)
        let bars_in_trade = state.current_idx.saturating_sub(self.entry_bar_idx);
        let effective_lookback = bars_in_trade.min(self.lookback);

        // Find extreme within the effective lookback
        let (high_n, low_n) = if effective_lookback > 0 {
            (
                state.highest_high(effective_lookback),
                state.lowest_low(effective_lookback),
            )
        } else {
            (position.high_since_entry, position.low_since_entry)
        };

        // Calculate new stop
        let new_stop = match direction {
            Direction::Long => high_n - (self.atr_multiplier * atr),
            Direction::Short => low_n + (self.atr_multiplier * atr),
        };

        // Only ratchet stop in favorable direction
        let should_update = match direction {
            Direction::Long => self.stop_price.map(|s| new_stop > s).unwrap_or(true),
            Direction::Short => self.stop_price.map(|s| new_stop < s).unwrap_or(true),
        };

        if should_update {
            self.stop_price = Some(new_stop);
            return Action::AdjustStop(new_stop);
        }

        // Check if stop is hit
        let stop = self.stop_price.unwrap_or(0.0);
        let stop_hit = match direction {
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
                name: "lookback".into(),
                param_type: ParamType::Int {
                    min: 5,
                    max: 55,
                    step: 5,
                },
                description: Some("Lookback period for highest/lowest".into()),
            },
            ParamDef {
                name: "atr_multiplier".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 5.0,
                    step: 0.5,
                },
                description: Some("ATR multiplier for stop distance".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.lookback, self.atr_multiplier))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.entry_bar_idx = 0;
        self.direction = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warmup() {
        let pm = ChandelierExit::default();
        assert_eq!(pm.name(), "ChandelierExit");
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = ChandelierExit::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }
}
