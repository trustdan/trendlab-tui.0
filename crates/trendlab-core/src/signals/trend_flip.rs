//! Trend Flip Signal Generator.
//!
//! Detects trend reversals based on price action relative to a moving average.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Trend Flip Signal Generator.
///
/// # Strategy
///
/// Identifies trend reversals when price flips from below to above a moving
/// average (or vice versa) with confirmation:
/// - Long when price closes above MA after being below for N consecutive bars
/// - Short when price closes below MA after being above for N consecutive bars
///
/// # Parameters
///
/// - `ma_period`: Moving average period (default: 50)
/// - `confirmation_bars`: Bars needed on one side before flip counts (default: 3)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct TrendFlip {
    ma_period: usize,
    confirmation_bars: usize,
    long_only: bool,
}

impl TrendFlip {
    /// Create a new trend flip signal generator.
    pub fn new(ma_period: usize, confirmation_bars: usize, long_only: bool) -> Self {
        Self {
            ma_period,
            confirmation_bars,
            long_only,
        }
    }

    /// Calculate SMA.
    fn sma(&self, bars: &[Bar], end_idx: usize) -> f64 {
        if end_idx + 1 < self.ma_period {
            return 0.0;
        }
        let start = end_idx + 1 - self.ma_period;
        let sum: f64 = bars[start..=end_idx].iter().map(|b| b.close).sum();
        sum / self.ma_period as f64
    }

    /// Count consecutive bars on one side of MA.
    fn count_side_bars(&self, bars: &[Bar], end_idx: usize, above: bool) -> usize {
        let mut count = 0;
        for i in (0..=end_idx).rev() {
            let ma = self.sma(bars, i);
            if ma <= 0.0 {
                break;
            }
            let is_above = bars[i].close > ma;
            if is_above == above {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}

impl Default for TrendFlip {
    fn default() -> Self {
        Self::new(50, 3, false)
    }
}

impl SignalGenerator for TrendFlip {
    fn name(&self) -> &str {
        "TrendFlip"
    }

    fn warmup_bars(&self) -> usize {
        self.ma_period + self.confirmation_bars
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.warmup_bars() {
            return None;
        }

        let ma_now = self.sma(state.bars, state.current_idx);
        let ma_prev = self.sma(state.bars, state.current_idx - 1);

        if ma_now <= 0.0 || ma_prev <= 0.0 {
            return None;
        }

        let prev_bar = state.prev_bar()?;
        let was_below = prev_bar.close < ma_prev;
        let is_above = bar.close > ma_now;

        // Bullish flip: was below, now above
        if was_below && is_above {
            // Check if we were below for confirmation_bars
            let below_count = self.count_side_bars(state.bars, state.current_idx - 1, false);
            if below_count >= self.confirmation_bars {
                let strength = (bar.close - ma_now) / ma_now;
                return Some(Signal::market(
                    Direction::Long,
                    strength.clamp(0.0, 1.0),
                    ma_now,
                ));
            }
        }

        // Bearish flip (if enabled): was above, now below
        if !self.long_only {
            let was_above = prev_bar.close > ma_prev;
            let is_below = bar.close < ma_now;

            if was_above && is_below {
                let above_count = self.count_side_bars(state.bars, state.current_idx - 1, true);
                if above_count >= self.confirmation_bars {
                    let strength = (ma_now - bar.close) / ma_now;
                    return Some(Signal::market(
                        Direction::Short,
                        strength.clamp(0.0, 1.0),
                        ma_now,
                    ));
                }
            }
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "ma_period".into(),
                param_type: ParamType::Int {
                    min: 20,
                    max: 200,
                    step: 10,
                },
                description: Some("Moving average period".into()),
            },
            ParamDef {
                name: "confirmation_bars".into(),
                param_type: ParamType::Int {
                    min: 1,
                    max: 10,
                    step: 1,
                },
                description: Some("Bars needed for confirmation".into()),
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
        let signal_gen = TrendFlip::new(50, 3, false);
        assert_eq!(signal_gen.warmup_bars(), 53);
    }

    #[test]
    fn test_sma() {
        let signal_gen = TrendFlip::new(3, 2, false);
        let prices = vec![100.0, 102.0, 104.0];
        let bars = make_bars(&prices);

        let sma = signal_gen.sma(&bars, 2);
        assert!((sma - 102.0).abs() < 0.01); // (100+102+104)/3 = 102
    }
}
