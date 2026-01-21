//! 52-Week Breakout Signal Generator.
//!
//! Generates long signals when price breaks above the 52-week (252 trading days) high.
//! This is the classic long-term breakout signal used by Turtle traders.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// 52-Week Breakout Signal Generator.
///
/// # Strategy
///
/// Long-term trend entry based on breaking annual highs:
/// - Long when price closes above the highest high of the last 252 bars
/// - Short when price closes below the lowest low of the last 252 bars
///
/// # Parameters
///
/// - `lookback`: Number of bars for the breakout (default: 252)
/// - `long_only`: Only generate long signals (default: true)
#[derive(Debug, Clone)]
pub struct FiftyTwoWeekBreakout {
    lookback: usize,
    long_only: bool,
}

impl FiftyTwoWeekBreakout {
    /// Create a new 52-week breakout signal generator.
    pub fn new(lookback: usize, long_only: bool) -> Self {
        Self { lookback, long_only }
    }
}

impl Default for FiftyTwoWeekBreakout {
    fn default() -> Self {
        Self::new(252, true)
    }
}

impl SignalGenerator for FiftyTwoWeekBreakout {
    fn name(&self) -> &str {
        "FiftyTwoWeekBreakout"
    }

    fn warmup_bars(&self) -> usize {
        self.lookback
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.lookback {
            return None;
        }

        let highest = state.highest_high(self.lookback);
        let lowest = state.lowest_low(self.lookback);

        // Long breakout
        if bar.close > highest {
            let strength = if highest > 0.0 {
                (bar.close - highest) / highest
            } else {
                0.0
            };
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                highest,
            ));
        }

        // Short breakdown (if enabled)
        if !self.long_only && bar.close < lowest {
            let strength = if lowest > 0.0 {
                (lowest - bar.close) / lowest
            } else {
                0.0
            };
            return Some(Signal::market(
                Direction::Short,
                strength.clamp(0.0, 1.0),
                lowest,
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "lookback".into(),
                param_type: ParamType::Int {
                    min: 126,
                    max: 504,
                    step: 21,
                },
                description: Some("Annual lookback period (trading days)".into()),
            },
            ParamDef {
                name: "long_only".into(),
                param_type: ParamType::Bool,
                description: Some("Only generate long signals".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn SignalGenerator> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bars(prices: &[f64]) -> Vec<Bar> {
        prices
            .iter()
            .enumerate()
            .map(|(i, &price)| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                    + chrono::Duration::days(i as i64),
                open: price - 0.5,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1_000_000,
                idx: i,
            })
            .collect()
    }

    #[test]
    fn test_warmup() {
        let signal_gen = FiftyTwoWeekBreakout::new(10, true);
        let bars = make_bars(&vec![100.0; 15]);
        let atr = vec![1.0; 15];
        let adx = vec![25.0; 15];

        for i in 0..10 {
            let state = MarketState::new(&bars[..=i], i, &atr[..=i], &adx[..=i]);
            assert!(signal_gen.generate(&bars[i], &state).is_none());
        }
    }

    #[test]
    fn test_breakout() {
        let signal_gen = FiftyTwoWeekBreakout::new(5, true);
        let mut prices: Vec<f64> = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0];
        prices.push(105.0); // Breakout

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = MarketState::new(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_some());
        assert_eq!(signal.unwrap().direction, Direction::Long);
    }
}
