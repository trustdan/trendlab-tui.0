//! ADX Trend Strength Filter.
//!
//! Only allows signals when ADX indicates a strong trend.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalFilter;
use crate::types::{Bar, Position, Signal};

/// ADX Trend Strength Filter.
///
/// # Strategy
///
/// Uses Average Directional Index (ADX) to gate entry signals:
/// - Only allow signals when ADX >= `min_adx` (strong trend)
/// - Optionally force exit when ADX drops below `exit_threshold`
///
/// ADX interpretation:
/// - 0-20: Weak/no trend (range-bound)
/// - 20-40: Developing trend
/// - 40-60: Strong trend
/// - 60+: Very strong trend
///
/// # Parameters
///
/// - `min_adx`: Minimum ADX to allow new entries (default: 25)
/// - `exit_threshold`: ADX below which to force exit (default: 15)
#[derive(Debug, Clone)]
pub struct AdxFilter {
    min_adx: f64,
    exit_threshold: f64,
}

impl AdxFilter {
    /// Create a new ADX filter.
    pub fn new(min_adx: f64, exit_threshold: f64) -> Self {
        Self {
            min_adx,
            exit_threshold,
        }
    }
}

impl Default for AdxFilter {
    fn default() -> Self {
        Self::new(25.0, 15.0)
    }
}

impl SignalFilter for AdxFilter {
    fn name(&self) -> &str {
        "AdxFilter"
    }

    fn allow_signal(&self, _signal: &Signal, _bar: &Bar, state: &MarketState) -> bool {
        state.current_adx() >= self.min_adx
    }

    fn force_exit(&self, _position: &Position, _bar: &Bar, state: &MarketState) -> bool {
        state.current_adx() < self.exit_threshold
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "min_adx".into(),
                param_type: ParamType::Float {
                    min: 15.0,
                    max: 40.0,
                    step: 5.0,
                },
                description: Some("Minimum ADX for entry".into()),
            },
            ParamDef {
                name: "exit_threshold".into(),
                param_type: ParamType::Float {
                    min: 10.0,
                    max: 25.0,
                    step: 5.0,
                },
                description: Some("ADX below which to force exit".into()),
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

    fn make_state(bars: &[Bar], adx: f64) -> MarketState<'_> {
        let atr = vec![1.0; bars.len()];
        let adx_vec = vec![adx; bars.len()];
        // Leak to get static lifetime for test (acceptable in tests)
        let atr_slice: &[f64] = Box::leak(atr.into_boxed_slice());
        let adx_slice: &[f64] = Box::leak(adx_vec.into_boxed_slice());
        MarketState::new(bars, bars.len() - 1, atr_slice, adx_slice)
    }

    #[test]
    fn test_name() {
        let filter = AdxFilter::default();
        assert_eq!(filter.name(), "AdxFilter");
    }

    #[test]
    fn test_allow_signal_above_threshold() {
        let filter = AdxFilter::new(25.0, 15.0);
        let bar = make_bar();
        let bars = [bar.clone()];
        let state = make_state(&bars, 30.0);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);

        assert!(filter.allow_signal(&signal, &bar, &state));
    }

    #[test]
    fn test_block_signal_below_threshold() {
        let filter = AdxFilter::new(25.0, 15.0);
        let bar = make_bar();
        let bars = [bar.clone()];
        let state = make_state(&bars, 20.0);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);

        assert!(!filter.allow_signal(&signal, &bar, &state));
    }

    #[test]
    fn test_force_exit_below_exit_threshold() {
        let filter = AdxFilter::new(25.0, 15.0);
        let bar = make_bar();
        let bars = [bar.clone()];
        let state = make_state(&bars, 10.0);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let position = Position::new(0, bar.date, 100.0, Direction::Long, 10000.0, signal);

        assert!(filter.force_exit(&position, &bar, &state));
    }

    #[test]
    fn test_no_force_exit_above_threshold() {
        let filter = AdxFilter::new(25.0, 15.0);
        let bar = make_bar();
        let bars = [bar.clone()];
        let state = make_state(&bars, 20.0);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let position = Position::new(0, bar.date, 100.0, Direction::Long, 10000.0, signal);

        assert!(!filter.force_exit(&position, &bar, &state));
    }
}
