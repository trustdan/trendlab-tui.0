//! Keltner Channel Exit Position Manager.
//!
//! Exits when price returns to the Keltner channel.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Keltner Channel Exit Position Manager.
///
/// # Strategy
///
/// Exit when price crosses back inside the Keltner channel:
/// - Long exit: price closes below the upper channel
/// - Short exit: price closes above the lower channel
///
/// Also maintains an ATR-based disaster stop.
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - stop trails from entry.
///
/// # Parameters
///
/// - `ema_period`: EMA period for channel center
/// - `atr_multiplier`: ATR multiplier for channel width
/// - `disaster_atr`: ATR multiplier for emergency stop
#[derive(Debug, Clone)]
pub struct KeltnerExit {
    ema_period: usize,
    atr_multiplier: f64,
    disaster_atr: f64,
    // Internal state
    stop_price: Option<f64>,
    high_since_entry: f64,
    low_since_entry: f64,
}

impl KeltnerExit {
    /// Create a new Keltner channel exit position manager.
    pub fn new(ema_period: usize, atr_multiplier: f64, disaster_atr: f64) -> Self {
        Self {
            ema_period,
            atr_multiplier,
            disaster_atr,
            stop_price: None,
            high_since_entry: 0.0,
            low_since_entry: f64::MAX,
        }
    }

    /// Calculate EMA of closing prices.
    fn calculate_ema(&self, bars: &[Bar], end_idx: usize) -> f64 {
        if end_idx + 1 < self.ema_period {
            return 0.0;
        }

        let multiplier = 2.0 / (self.ema_period as f64 + 1.0);
        let start = end_idx + 1 - self.ema_period;

        let initial_sma: f64 = bars[start..start + self.ema_period]
            .iter()
            .map(|b| b.close)
            .sum::<f64>()
            / self.ema_period as f64;

        let mut ema = initial_sma;
        for i in start + self.ema_period..=end_idx {
            ema = (bars[i].close - ema) * multiplier + ema;
        }
        ema
    }
}

impl Default for KeltnerExit {
    fn default() -> Self {
        Self::new(20, 2.0, 4.0)
    }
}

impl PositionManager for KeltnerExit {
    fn name(&self) -> &str {
        "KeltnerExit"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.high_since_entry = entry_price;
        self.low_since_entry = entry_price;

        let bar_range = entry_bar.high - entry_bar.low;
        let initial_atr = if bar_range > 0.0 { bar_range } else { 1.0 };

        // Set disaster stop
        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price - (self.disaster_atr * initial_atr),
            Direction::Short => entry_price + (self.disaster_atr * initial_atr),
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

        // Calculate Keltner channel
        let ema = self.calculate_ema(state.bars, state.current_idx);
        if ema <= 0.0 {
            return Action::Hold;
        }

        let upper = ema + self.atr_multiplier * atr;
        let lower = ema - self.atr_multiplier * atr;

        // Check channel exit condition
        let channel_exit = match position.direction {
            Direction::Long => bar.close < upper, // Back inside channel
            Direction::Short => bar.close > lower,
        };

        if channel_exit {
            return Action::Exit(ExitReason::SignalExit);
        }

        // Update trailing disaster stop
        let new_stop = match position.direction {
            Direction::Long => self.high_since_entry - (self.disaster_atr * atr),
            Direction::Short => self.low_since_entry + (self.disaster_atr * atr),
        };

        let should_update = match position.direction {
            Direction::Long => self.stop_price.map(|s| new_stop > s).unwrap_or(true),
            Direction::Short => self.stop_price.map(|s| new_stop < s).unwrap_or(true),
        };

        if should_update {
            self.stop_price = Some(new_stop);
        }

        // Check disaster stop
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
                name: "ema_period".into(),
                param_type: ParamType::Int {
                    min: 10,
                    max: 50,
                    step: 5,
                },
                description: Some("EMA period for channel".into()),
            },
            ParamDef {
                name: "atr_multiplier".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 3.0,
                    step: 0.5,
                },
                description: Some("ATR multiplier for channel".into()),
            },
            ParamDef {
                name: "disaster_atr".into(),
                param_type: ParamType::Float {
                    min: 3.0,
                    max: 6.0,
                    step: 0.5,
                },
                description: Some("ATR multiplier for disaster stop".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(
            self.ema_period,
            self.atr_multiplier,
            self.disaster_atr,
        ))
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
    fn test_name() {
        let pm = KeltnerExit::default();
        assert_eq!(pm.name(), "KeltnerExit");
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = KeltnerExit::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }
}
