//! Moving Average Crossover Signal Generator.
//!
//! Generates signals when a fast MA crosses above/below a slow MA.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Moving Average Crossover Signal Generator.
///
/// # Strategy
///
/// Classic trend-following crossover system:
/// - Long when fast MA crosses above slow MA
/// - Short when fast MA crosses below slow MA
///
/// # Parameters
///
/// - `fast_period`: Fast MA period (default: 10)
/// - `slow_period`: Slow MA period (default: 50)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct MaCrossover {
    fast_period: usize,
    slow_period: usize,
    long_only: bool,
}

impl MaCrossover {
    /// Create a new MA crossover signal generator.
    pub fn new(fast_period: usize, slow_period: usize, long_only: bool) -> Self {
        Self {
            fast_period,
            slow_period,
            long_only,
        }
    }

    /// Compute SMA for a slice of bars.
    fn sma(bars: &[Bar], end_idx: usize, period: usize) -> f64 {
        if end_idx + 1 < period {
            return 0.0;
        }
        let start = end_idx + 1 - period;
        let sum: f64 = bars[start..=end_idx].iter().map(|b| b.close).sum();
        sum / period as f64
    }
}

impl Default for MaCrossover {
    fn default() -> Self {
        Self::new(10, 50, false)
    }
}

impl SignalGenerator for MaCrossover {
    fn name(&self) -> &str {
        "MaCrossover"
    }

    fn warmup_bars(&self) -> usize {
        self.slow_period + 1 // Need extra bar for crossover detection
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.slow_period {
            return None;
        }

        // Current MAs
        let fast_now = Self::sma(state.bars, state.current_idx, self.fast_period);
        let slow_now = Self::sma(state.bars, state.current_idx, self.slow_period);

        // Previous MAs
        let fast_prev = Self::sma(state.bars, state.current_idx - 1, self.fast_period);
        let slow_prev = Self::sma(state.bars, state.current_idx - 1, self.slow_period);

        // Golden cross: fast crosses above slow
        if fast_prev <= slow_prev && fast_now > slow_now {
            let strength = (fast_now - slow_now) / slow_now;
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                bar.close,
            ));
        }

        // Death cross: fast crosses below slow (if enabled)
        if !self.long_only && fast_prev >= slow_prev && fast_now < slow_now {
            let strength = (slow_now - fast_now) / slow_now;
            return Some(Signal::market(
                Direction::Short,
                strength.clamp(0.0, 1.0),
                bar.close,
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "fast_period".into(),
                param_type: ParamType::Int {
                    min: 5,
                    max: 50,
                    step: 5,
                },
                description: Some("Fast MA period".into()),
            },
            ParamDef {
                name: "slow_period".into(),
                param_type: ParamType::Int {
                    min: 20,
                    max: 200,
                    step: 10,
                },
                description: Some("Slow MA period".into()),
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
    fn test_golden_cross() {
        let signal_gen = MaCrossover::new(3, 5, false);

        // Create a scenario where 3-bar MA crosses above 5-bar MA:
        // Downtrend keeps slow MA high and fast MA low, then big reversal
        // Bar 6: fast(100) < slow(102), Bar 7: fast(109.33) > slow(106.8)
        let prices: Vec<f64> = vec![110.0, 108.0, 106.0, 104.0, 102.0, 100.0, 98.0, 130.0];
        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = MarketState::new(&bars, 7, &atr, &adx);
        let signal = signal_gen.generate(&bars[7], &state);

        assert!(signal.is_some());
        assert_eq!(signal.unwrap().direction, Direction::Long);
    }
}
