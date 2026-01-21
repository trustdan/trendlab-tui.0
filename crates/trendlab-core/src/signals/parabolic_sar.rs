//! Parabolic SAR Signal Generator.
//!
//! Generates signals based on the Parabolic Stop and Reverse indicator.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Parabolic SAR Signal Generator.
///
/// # Strategy
///
/// The Parabolic SAR (Stop And Reverse) is a trend-following indicator:
/// - Long when price crosses above the SAR
/// - Short when price crosses below the SAR
///
/// # Parameters
///
/// - `af_start`: Initial acceleration factor (default: 0.02)
/// - `af_max`: Maximum acceleration factor (default: 0.20)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct ParabolicSar {
    af_start: f64,
    af_max: f64,
    long_only: bool,
}

impl ParabolicSar {
    /// Create a new Parabolic SAR signal generator.
    pub fn new(af_start: f64, af_max: f64, long_only: bool) -> Self {
        Self {
            af_start,
            af_max,
            long_only,
        }
    }

    /// Calculate SAR value for current bar.
    /// Returns (sar, is_bullish).
    fn calculate_sar(&self, bars: &[Bar], end_idx: usize) -> Option<(f64, bool)> {
        if bars.len() < 3 || end_idx < 2 {
            return None;
        }

        // Simple SAR calculation (simplified version)
        let mut af = self.af_start;
        let mut is_bullish = bars[1].close > bars[0].close;
        let mut sar = if is_bullish { bars[0].low } else { bars[0].high };
        let mut ep = if is_bullish { bars[1].high } else { bars[1].low };

        for i in 2..=end_idx {
            let bar = &bars[i];
            let prev_bar = &bars[i - 1];

            // Calculate new SAR
            let new_sar = sar + af * (ep - sar);

            // Check for reversal
            if is_bullish {
                // Ensure SAR is below last two lows
                let clamped_sar = new_sar.min(prev_bar.low).min(bars[i - 2].low);

                if bar.low < clamped_sar {
                    // Reversal to bearish
                    is_bullish = false;
                    sar = ep;
                    ep = bar.low;
                    af = self.af_start;
                } else {
                    sar = clamped_sar;
                    if bar.high > ep {
                        ep = bar.high;
                        af = (af + self.af_start).min(self.af_max);
                    }
                }
            } else {
                // Ensure SAR is above last two highs
                let clamped_sar = new_sar.max(prev_bar.high).max(bars[i - 2].high);

                if bar.high > clamped_sar {
                    // Reversal to bullish
                    is_bullish = true;
                    sar = ep;
                    ep = bar.high;
                    af = self.af_start;
                } else {
                    sar = clamped_sar;
                    if bar.low < ep {
                        ep = bar.low;
                        af = (af + self.af_start).min(self.af_max);
                    }
                }
            }
        }

        Some((sar, is_bullish))
    }
}

impl Default for ParabolicSar {
    fn default() -> Self {
        Self::new(0.02, 0.20, false)
    }
}

impl SignalGenerator for ParabolicSar {
    fn name(&self) -> &str {
        "ParabolicSar"
    }

    fn warmup_bars(&self) -> usize {
        5 // Need a few bars to establish trend
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < 5 {
            return None;
        }

        // Get current and previous SAR
        let (sar_now, is_bull_now) = self.calculate_sar(state.bars, state.current_idx)?;
        let (_, is_bull_prev) = self.calculate_sar(state.bars, state.current_idx - 1)?;

        // Check for trend reversal
        if !is_bull_prev && is_bull_now {
            // Bullish reversal
            let strength = (bar.close - sar_now) / bar.close;
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                sar_now,
            ));
        }

        if !self.long_only && is_bull_prev && !is_bull_now {
            // Bearish reversal
            let strength = (sar_now - bar.close) / bar.close;
            return Some(Signal::market(
                Direction::Short,
                strength.clamp(0.0, 1.0),
                sar_now,
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "af_start".into(),
                param_type: ParamType::Float {
                    min: 0.01,
                    max: 0.05,
                    step: 0.01,
                },
                description: Some("Initial acceleration factor".into()),
            },
            ParamDef {
                name: "af_max".into(),
                param_type: ParamType::Float {
                    min: 0.10,
                    max: 0.30,
                    step: 0.05,
                },
                description: Some("Maximum acceleration factor".into()),
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
        let signal_gen = ParabolicSar::default();
        assert_eq!(signal_gen.warmup_bars(), 5);
    }

    #[test]
    fn test_sar_calculation() {
        let signal_gen = ParabolicSar::default();
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        let bars = make_bars(&prices);

        let result = signal_gen.calculate_sar(&bars, 5);
        assert!(result.is_some());
        let (sar, is_bullish) = result.unwrap();
        assert!(is_bullish); // Uptrend
        assert!(sar < 105.0); // SAR should be below current price in uptrend
    }
}
