//! Moving Average Regime Filter.
//!
//! Only allows signals when price is on the correct side of a moving average.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalFilter;
use crate::types::{Bar, Direction, Position, Signal};

/// Moving Average Regime Filter.
///
/// # Strategy
///
/// Uses price relative to a moving average to gate signals:
/// - Long signals only when price > MA (bullish regime)
/// - Short signals only when price < MA (bearish regime)
/// - Force exit when regime changes against position
///
/// # Parameters
///
/// - `period`: MA lookback period (default: 200)
/// - `require_alignment`: If true, only allow aligned signals (default: true)
#[derive(Debug, Clone)]
pub struct MaRegimeFilter {
    period: usize,
    require_alignment: bool,
}

impl MaRegimeFilter {
    /// Create a new MA regime filter.
    pub fn new(period: usize, require_alignment: bool) -> Self {
        Self {
            period,
            require_alignment,
        }
    }

    fn calculate_sma(&self, bars: &[Bar], end_idx: usize) -> Option<f64> {
        if end_idx + 1 < self.period {
            return None;
        }

        let start = end_idx + 1 - self.period;
        let sum: f64 = bars[start..=end_idx].iter().map(|b| b.close).sum();
        Some(sum / self.period as f64)
    }
}

impl Default for MaRegimeFilter {
    fn default() -> Self {
        Self::new(200, true)
    }
}

impl SignalFilter for MaRegimeFilter {
    fn name(&self) -> &str {
        "MaRegimeFilter"
    }

    fn allow_signal(&self, signal: &Signal, _bar: &Bar, state: &MarketState) -> bool {
        if !self.require_alignment {
            return true;
        }

        let Some(ma) = self.calculate_sma(state.bars, state.current_idx) else {
            // Not enough data for MA, allow signal
            return true;
        };

        let current_close = state.bars[state.current_idx].close;

        match signal.direction {
            Direction::Long => current_close > ma,
            Direction::Short => current_close < ma,
        }
    }

    fn force_exit(&self, position: &Position, _bar: &Bar, state: &MarketState) -> bool {
        let Some(ma) = self.calculate_sma(state.bars, state.current_idx) else {
            return false;
        };

        let current_close = state.bars[state.current_idx].close;

        // Force exit if regime changes against position
        match position.direction {
            Direction::Long => current_close < ma,
            Direction::Short => current_close > ma,
        }
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "period".into(),
                param_type: ParamType::Int {
                    min: 50,
                    max: 250,
                    step: 50,
                },
                description: Some("MA lookback period".into()),
            },
            ParamDef {
                name: "require_alignment".into(),
                param_type: ParamType::Bool,
                description: Some("Require price/MA alignment".into()),
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
    use chrono::NaiveDate;

    fn make_bars(prices: &[f64]) -> Vec<Bar> {
        prices
            .iter()
            .enumerate()
            .map(|(i, &close)| {
                Bar::new(
                    NaiveDate::from_ymd_opt(2024, 1, 1)
                        .unwrap()
                        .checked_add_signed(chrono::Duration::days(i as i64))
                        .unwrap(),
                    close - 1.0,
                    close + 1.0,
                    close - 2.0,
                    close,
                    1_000_000,
                    i,
                )
            })
            .collect()
    }

    fn make_state(bars: &[Bar]) -> MarketState<'_> {
        let len = bars.len();
        let atr = vec![1.0; len];
        let adx = vec![25.0; len];
        let atr_slice: &[f64] = Box::leak(atr.into_boxed_slice());
        let adx_slice: &[f64] = Box::leak(adx.into_boxed_slice());
        MarketState::new(bars, len - 1, atr_slice, adx_slice)
    }

    #[test]
    fn test_name() {
        let filter = MaRegimeFilter::default();
        assert_eq!(filter.name(), "MaRegimeFilter");
    }

    #[test]
    fn test_allow_long_above_ma() {
        // 5-period MA for easier testing
        let filter = MaRegimeFilter::new(5, true);
        // Prices: [100, 100, 100, 100, 100, 110] - MA = 100, current = 110
        let prices = vec![100.0, 100.0, 100.0, 100.0, 100.0, 110.0];
        let bars = make_bars(&prices);
        let state = make_state(&bars);
        let signal = Signal::market(Direction::Long, 1.0, 110.0);

        assert!(filter.allow_signal(&signal, &bars.last().unwrap(), &state));
    }

    #[test]
    fn test_block_long_below_ma() {
        let filter = MaRegimeFilter::new(5, true);
        // Prices: [100, 100, 100, 100, 100, 90] - MA = 100, current = 90
        let prices = vec![100.0, 100.0, 100.0, 100.0, 100.0, 90.0];
        let bars = make_bars(&prices);
        let state = make_state(&bars);
        let signal = Signal::market(Direction::Long, 1.0, 90.0);

        assert!(!filter.allow_signal(&signal, &bars.last().unwrap(), &state));
    }

    #[test]
    fn test_force_exit_regime_change() {
        let filter = MaRegimeFilter::new(5, true);
        // Price drops below MA while in long position
        let prices = vec![100.0, 100.0, 100.0, 100.0, 100.0, 90.0];
        let bars = make_bars(&prices);
        let state = make_state(&bars);
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let position = Position::new(
            0,
            bars[0].date,
            100.0,
            Direction::Long,
            10000.0,
            signal,
        );

        assert!(filter.force_exit(&position, &bars.last().unwrap(), &state));
    }
}
