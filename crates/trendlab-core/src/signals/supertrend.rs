//! Supertrend Signal Generator.
//!
//! Generates signals based on the Supertrend indicator, which uses ATR bands.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Supertrend Signal Generator.
///
/// # Strategy
///
/// The Supertrend indicator creates dynamic support/resistance levels:
/// - Long when price closes above the upper Supertrend line
/// - Short when price closes below the lower Supertrend line
///
/// # Parameters
///
/// - `atr_multiplier`: Multiplier for ATR bands (default: 3.0)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct Supertrend {
    atr_multiplier: f64,
    long_only: bool,
}

impl Supertrend {
    /// Create a new Supertrend signal generator.
    pub fn new(atr_multiplier: f64, long_only: bool) -> Self {
        Self {
            atr_multiplier,
            long_only,
        }
    }

    /// Calculate Supertrend bands.
    fn calculate_bands(&self, bar: &Bar, atr: f64) -> (f64, f64) {
        let hl2 = (bar.high + bar.low) / 2.0;
        let upper = hl2 + self.atr_multiplier * atr;
        let lower = hl2 - self.atr_multiplier * atr;
        (upper, lower)
    }
}

impl Default for Supertrend {
    fn default() -> Self {
        Self::new(3.0, false)
    }
}

impl SignalGenerator for Supertrend {
    fn name(&self) -> &str {
        "Supertrend"
    }

    fn warmup_bars(&self) -> usize {
        14 // ATR needs 14 bars
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < 14 {
            return None;
        }

        let atr = state.current_atr();
        if atr <= 0.0 {
            return None;
        }

        let (upper, lower) = self.calculate_bands(bar, atr);

        // Check previous bar for trend detection
        let prev_bar = state.prev_bar()?;
        let prev_atr = state.atr_ago(1)?;
        let (prev_upper, prev_lower) = self.calculate_bands(prev_bar, prev_atr);

        // Bullish crossover: price crosses above upper band
        if prev_bar.close <= prev_upper && bar.close > upper {
            let strength = (bar.close - upper) / atr;
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                lower, // Support level
            ));
        }

        // Bearish crossover (if enabled)
        if !self.long_only && prev_bar.close >= prev_lower && bar.close < lower {
            let strength = (lower - bar.close) / atr;
            return Some(Signal::market(
                Direction::Short,
                strength.clamp(0.0, 1.0),
                upper, // Resistance level
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "atr_multiplier".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 5.0,
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

    fn make_bar(idx: usize, close: f64, high: f64, low: f64) -> Bar {
        Bar {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            open: close - 0.5,
            high,
            low,
            close,
            volume: 1_000_000,
            idx,
        }
    }

    #[test]
    fn test_warmup() {
        let signal_gen = Supertrend::default();
        assert_eq!(signal_gen.warmup_bars(), 14);
    }

    #[test]
    fn test_bands_calculation() {
        let signal_gen = Supertrend::new(2.0, false);
        let bar = make_bar(0, 100.0, 102.0, 98.0);
        let (upper, lower) = signal_gen.calculate_bands(&bar, 1.0);

        // hl2 = (102 + 98) / 2 = 100
        // upper = 100 + 2 * 1 = 102
        // lower = 100 - 2 * 1 = 98
        assert!((upper - 102.0).abs() < 0.001);
        assert!((lower - 98.0).abs() < 0.001);
    }
}
