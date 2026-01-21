//! Volatility Filter.
//!
//! Filters signals based on ATR relative to its historical average.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalFilter;
use crate::types::{Bar, Position, Signal};

/// Volatility Regime Filter.
///
/// # Strategy
///
/// Uses ATR relative to its own moving average to classify volatility regime:
/// - ATR ratio = current ATR / SMA(ATR, lookback)
/// - Low volatility: ratio < low_threshold (e.g., 0.7)
/// - High volatility: ratio > high_threshold (e.g., 1.5)
/// - Normal: between thresholds
///
/// Can be configured to:
/// - Block signals in high volatility (whipsaw risk)
/// - Block signals in low volatility (no moves)
/// - Or both (only trade normal volatility)
///
/// # Parameters
///
/// - `lookback`: Bars to average ATR over (default: 20)
/// - `low_threshold`: Ratio below which is "low vol" (default: 0.7)
/// - `high_threshold`: Ratio above which is "high vol" (default: 1.5)
/// - `allow_low`: Allow signals in low volatility (default: true)
/// - `allow_high`: Allow signals in high volatility (default: true)
#[derive(Debug, Clone)]
pub struct VolatilityFilter {
    lookback: usize,
    low_threshold: f64,
    high_threshold: f64,
    allow_low: bool,
    allow_high: bool,
}

impl VolatilityFilter {
    /// Create a new volatility filter.
    pub fn new(
        lookback: usize,
        low_threshold: f64,
        high_threshold: f64,
        allow_low: bool,
        allow_high: bool,
    ) -> Self {
        Self {
            lookback,
            low_threshold,
            high_threshold,
            allow_low,
            allow_high,
        }
    }

    /// Calculate volatility ratio (current ATR / average ATR).
    fn volatility_ratio(&self, state: &MarketState) -> Option<f64> {
        let current_atr = state.current_atr();
        if current_atr <= 0.0 {
            return None;
        }

        // Calculate average ATR over lookback using atr_ago
        let mut sum = 0.0;
        let mut count = 0;
        for n in 0..self.lookback {
            if let Some(atr) = state.atr_ago(n) {
                sum += atr;
                count += 1;
            }
        }

        if count < self.lookback {
            return None; // Not enough data
        }

        let avg_atr = sum / count as f64;
        if avg_atr <= 0.0 {
            return None;
        }

        Some(current_atr / avg_atr)
    }
}

impl Default for VolatilityFilter {
    fn default() -> Self {
        // Default: only block high volatility (whipsaw protection)
        Self::new(20, 0.7, 1.5, true, false)
    }
}

impl SignalFilter for VolatilityFilter {
    fn name(&self) -> &str {
        "VolatilityFilter"
    }

    fn allow_signal(&self, _signal: &Signal, _bar: &Bar, state: &MarketState) -> bool {
        let Some(ratio) = self.volatility_ratio(state) else {
            // Not enough data, allow signal
            return true;
        };

        // Check if in low vol regime
        if ratio < self.low_threshold && !self.allow_low {
            return false;
        }

        // Check if in high vol regime
        if ratio > self.high_threshold && !self.allow_high {
            return false;
        }

        true
    }

    fn force_exit(&self, _position: &Position, _bar: &Bar, _state: &MarketState) -> bool {
        // This filter doesn't force exits, just blocks entries
        false
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "lookback".into(),
                param_type: ParamType::Int {
                    min: 10,
                    max: 50,
                    step: 10,
                },
                description: Some("ATR averaging period".into()),
            },
            ParamDef {
                name: "low_threshold".into(),
                param_type: ParamType::Float {
                    min: 0.5,
                    max: 0.9,
                    step: 0.1,
                },
                description: Some("Ratio for low volatility".into()),
            },
            ParamDef {
                name: "high_threshold".into(),
                param_type: ParamType::Float {
                    min: 1.2,
                    max: 2.0,
                    step: 0.1,
                },
                description: Some("Ratio for high volatility".into()),
            },
            ParamDef {
                name: "allow_low".into(),
                param_type: ParamType::Bool,
                description: Some("Allow signals in low vol".into()),
            },
            ParamDef {
                name: "allow_high".into(),
                param_type: ParamType::Bool,
                description: Some("Allow signals in high vol".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn SignalFilter> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Direction;
    use chrono::NaiveDate;

    fn make_bar() -> Bar {
        Bar::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            105.0,
            95.0,
            102.0,
            1_000_000,
            0,
        )
    }

    fn make_bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                Bar::new(
                    NaiveDate::from_ymd_opt(2024, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(i as i64))
                        .unwrap(),
                    100.0,
                    105.0,
                    95.0,
                    102.0,
                    1_000_000,
                    i,
                )
            })
            .collect()
    }

    fn make_state_with_atr<'a>(bars: &'a [Bar], atr_values: &[f64]) -> MarketState<'a> {
        let adx = vec![25.0; bars.len()];
        let atr_slice: &'a [f64] = Box::leak(atr_values.to_vec().into_boxed_slice());
        let adx_slice: &'a [f64] = Box::leak(adx.into_boxed_slice());
        MarketState::new(bars, bars.len() - 1, atr_slice, adx_slice)
    }

    #[test]
    fn test_name() {
        let filter = VolatilityFilter::default();
        assert_eq!(filter.name(), "VolatilityFilter");
    }

    #[test]
    fn test_allow_normal_volatility() {
        let filter = VolatilityFilter::new(5, 0.7, 1.5, true, true);
        let bars = make_bars(10);
        // Constant ATR = ratio of 1.0 (normal)
        let atr = vec![1.0; 10];
        let state = make_state_with_atr(&bars, &atr);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);

        assert!(filter.allow_signal(&signal, &make_bar(), &state));
    }

    #[test]
    fn test_block_high_volatility() {
        // Block high vol, allow low vol
        let filter = VolatilityFilter::new(5, 0.7, 1.5, true, false);
        let bars = make_bars(10);
        // High ATR at end: ratio = 2.0 / 1.0 = 2.0 (> 1.5)
        let mut atr = vec![1.0; 10];
        atr[9] = 2.0;
        let state = make_state_with_atr(&bars, &atr);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);

        assert!(!filter.allow_signal(&signal, &make_bar(), &state));
    }

    #[test]
    fn test_block_low_volatility() {
        // Allow high vol, block low vol
        let filter = VolatilityFilter::new(5, 0.7, 1.5, false, true);
        let bars = make_bars(10);
        // Low ATR at end: ratio = 0.5 / 1.0 = 0.5 (< 0.7)
        let mut atr = vec![1.0; 10];
        atr[9] = 0.5;
        let state = make_state_with_atr(&bars, &atr);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);

        assert!(!filter.allow_signal(&signal, &make_bar(), &state));
    }

    #[test]
    fn test_never_force_exit() {
        let filter = VolatilityFilter::default();
        let bars = make_bars(10);
        let atr = vec![1.0; 10];
        let state = make_state_with_atr(&bars, &atr);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let position = Position::new(
            0,
            bars[0].date,
            100.0,
            Direction::Long,
            10000.0,
            signal,
        );

        assert!(!filter.force_exit(&position, &make_bar(), &state));
    }
}
