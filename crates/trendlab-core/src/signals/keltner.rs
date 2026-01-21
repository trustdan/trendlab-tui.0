//! Keltner Channel Breakout Signal Generator.
//!
//! Similar to Bollinger Bands but uses ATR instead of standard deviation.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Keltner Channel Breakout Signal Generator.
///
/// # Strategy
///
/// Keltner Channels use ATR for band width, making them less sensitive to
/// price spikes than Bollinger Bands:
/// - Long when price closes above the upper channel
/// - Short when price closes below the lower channel
///
/// # Parameters
///
/// - `ema_period`: EMA period for middle line (default: 20)
/// - `atr_multiplier`: ATR multiplier for bands (default: 2.0)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct KeltnerBreakout {
    ema_period: usize,
    atr_multiplier: f64,
    long_only: bool,
}

impl KeltnerBreakout {
    /// Create a new Keltner channel breakout signal generator.
    pub fn new(ema_period: usize, atr_multiplier: f64, long_only: bool) -> Self {
        Self {
            ema_period,
            atr_multiplier,
            long_only,
        }
    }

    /// Calculate simple EMA for channel midline.
    fn calculate_ema(&self, bars: &[Bar], end_idx: usize) -> f64 {
        if end_idx + 1 < self.ema_period {
            return 0.0;
        }

        let multiplier = 2.0 / (self.ema_period as f64 + 1.0);
        let start = end_idx + 1 - self.ema_period;

        // Start with SMA
        let initial_sma: f64 = bars[start..start + self.ema_period]
            .iter()
            .map(|b| b.close)
            .sum::<f64>() / self.ema_period as f64;

        // Apply EMA smoothing (simplified for remaining bars if any)
        let mut ema = initial_sma;
        for i in start + self.ema_period..=end_idx {
            ema = (bars[i].close - ema) * multiplier + ema;
        }
        ema
    }
}

impl Default for KeltnerBreakout {
    fn default() -> Self {
        Self::new(20, 2.0, false)
    }
}

impl SignalGenerator for KeltnerBreakout {
    fn name(&self) -> &str {
        "KeltnerBreakout"
    }

    fn warmup_bars(&self) -> usize {
        self.ema_period.max(14) // Max of EMA period and ATR warmup
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.warmup_bars() {
            return None;
        }

        let middle = self.calculate_ema(state.bars, state.current_idx);
        let atr = state.current_atr();

        if atr <= 0.0 || middle <= 0.0 {
            return None;
        }

        let upper = middle + self.atr_multiplier * atr;
        let lower = middle - self.atr_multiplier * atr;

        // Previous bar for crossover detection
        let prev_bar = state.prev_bar()?;
        let prev_middle = self.calculate_ema(state.bars, state.current_idx - 1);
        let prev_atr = state.atr_ago(1)?;
        let prev_upper = prev_middle + self.atr_multiplier * prev_atr;
        let prev_lower = prev_middle - self.atr_multiplier * prev_atr;

        // Long breakout
        if prev_bar.close <= prev_upper && bar.close > upper {
            let strength = (bar.close - upper) / atr;
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                lower,
            ));
        }

        // Short breakdown (if enabled)
        if !self.long_only && prev_bar.close >= prev_lower && bar.close < lower {
            let strength = (lower - bar.close) / atr;
            return Some(Signal::market(
                Direction::Short,
                strength.clamp(0.0, 1.0),
                upper,
            ));
        }

        None
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
                description: Some("EMA period for middle line".into()),
            },
            ParamDef {
                name: "atr_multiplier".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 4.0,
                    step: 0.5,
                },
                description: Some("ATR multiplier for bands".into()),
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
        let signal_gen = KeltnerBreakout::new(20, 2.0, false);
        assert_eq!(signal_gen.warmup_bars(), 20);

        let signal_gen2 = KeltnerBreakout::new(10, 2.0, false);
        assert_eq!(signal_gen2.warmup_bars(), 14); // ATR warmup is larger
    }

    #[test]
    fn test_ema_calculation() {
        let signal_gen = KeltnerBreakout::new(5, 2.0, false);
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0];
        let bars = make_bars(&prices);

        let ema = signal_gen.calculate_ema(&bars, 4);
        // EMA should be close to SMA for short series
        assert!(ema > 100.0 && ema < 105.0);
    }
}
