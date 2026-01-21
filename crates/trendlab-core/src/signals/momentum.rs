//! Momentum (TSMOM) Signal Generator.
//!
//! Time-series momentum signal based on past returns.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Momentum (TSMOM) Signal Generator.
///
/// # Strategy
///
/// Time-series momentum exploits the tendency of assets to continue their
/// recent performance:
/// - Long when N-period return is positive
/// - Short when N-period return is negative
///
/// # Parameters
///
/// - `lookback`: Number of bars for momentum calculation (default: 252)
/// - `threshold`: Minimum return threshold for signal (default: 0.0)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct Momentum {
    lookback: usize,
    threshold: f64,
    long_only: bool,
}

impl Momentum {
    /// Create a new momentum signal generator.
    pub fn new(lookback: usize, threshold: f64, long_only: bool) -> Self {
        Self {
            lookback,
            threshold,
            long_only,
        }
    }

    /// Calculate N-period return.
    fn calculate_return(&self, current_close: f64, past_close: f64) -> f64 {
        if past_close <= 0.0 {
            return 0.0;
        }
        (current_close - past_close) / past_close
    }
}

impl Default for Momentum {
    fn default() -> Self {
        Self::new(252, 0.0, false)
    }
}

impl SignalGenerator for Momentum {
    fn name(&self) -> &str {
        "Momentum"
    }

    fn warmup_bars(&self) -> usize {
        self.lookback
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.lookback {
            return None;
        }

        let past_bar = state.bar_ago(self.lookback)?;
        let ret = self.calculate_return(bar.close, past_bar.close);

        // Long signal: positive momentum above threshold
        if ret > self.threshold {
            return Some(Signal::market(
                Direction::Long,
                ret.min(1.0),
                bar.close,
            ));
        }

        // Short signal (if enabled): negative momentum below threshold
        if !self.long_only && ret < -self.threshold {
            return Some(Signal::market(
                Direction::Short,
                ret.abs().min(1.0),
                bar.close,
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "lookback".into(),
                param_type: ParamType::Int {
                    min: 21,
                    max: 504,
                    step: 21,
                },
                description: Some("Momentum lookback period".into()),
            },
            ParamDef {
                name: "threshold".into(),
                param_type: ParamType::Float {
                    min: 0.0,
                    max: 0.2,
                    step: 0.02,
                },
                description: Some("Minimum return threshold".into()),
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
    fn test_positive_momentum() {
        let signal_gen = Momentum::new(5, 0.0, false);

        // Prices going up: 100 -> 120 = 20% return
        let mut prices = vec![100.0; 6];
        prices.push(120.0);

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = MarketState::new(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_some());
        assert_eq!(signal.unwrap().direction, Direction::Long);
    }

    #[test]
    fn test_negative_momentum() {
        let signal_gen = Momentum::new(5, 0.0, false);

        // Prices going down: 100 -> 80 = -20% return
        let mut prices = vec![100.0; 6];
        prices.push(80.0);

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = MarketState::new(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_some());
        assert_eq!(signal.unwrap().direction, Direction::Short);
    }

    #[test]
    fn test_long_only() {
        let signal_gen = Momentum::new(5, 0.0, true);

        let mut prices = vec![100.0; 6];
        prices.push(80.0);

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = MarketState::new(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        // Should not generate short signal in long_only mode
        assert!(signal.is_none());
    }
}
