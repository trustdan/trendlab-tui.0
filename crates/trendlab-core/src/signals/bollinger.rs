//! Bollinger Bands Breakout Signal Generator.
//!
//! Generates signals when price breaks out of Bollinger Bands.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Bollinger Bands Breakout Signal Generator.
///
/// # Strategy
///
/// Bollinger Bands breakout captures volatility expansion:
/// - Long when price closes above the upper band
/// - Short when price closes below the lower band
///
/// # Parameters
///
/// - `period`: SMA period for middle band (default: 20)
/// - `std_dev`: Standard deviation multiplier (default: 2.0)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct BollingerBreakout {
    period: usize,
    std_dev: f64,
    long_only: bool,
}

impl BollingerBreakout {
    /// Create a new Bollinger breakout signal generator.
    pub fn new(period: usize, std_dev: f64, long_only: bool) -> Self {
        Self {
            period,
            std_dev,
            long_only,
        }
    }

    /// Calculate Bollinger Bands.
    fn calculate_bands(&self, bars: &[Bar], end_idx: usize) -> Option<(f64, f64, f64)> {
        if end_idx + 1 < self.period {
            return None;
        }

        let start = end_idx + 1 - self.period;
        let closes: Vec<f64> = bars[start..=end_idx].iter().map(|b| b.close).collect();

        let mean: f64 = closes.iter().sum::<f64>() / closes.len() as f64;
        let variance: f64 = closes.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / closes.len() as f64;
        let std = variance.sqrt();

        let upper = mean + self.std_dev * std;
        let lower = mean - self.std_dev * std;

        Some((upper, mean, lower))
    }
}

impl Default for BollingerBreakout {
    fn default() -> Self {
        Self::new(20, 2.0, false)
    }
}

impl SignalGenerator for BollingerBreakout {
    fn name(&self) -> &str {
        "BollingerBreakout"
    }

    fn warmup_bars(&self) -> usize {
        self.period
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.period {
            return None;
        }

        let (upper, _middle, lower) = self.calculate_bands(state.bars, state.current_idx)?;

        // Check previous bar for breakout confirmation
        let prev_bar = state.prev_bar()?;

        // Long breakout: cross above upper band
        if prev_bar.close <= upper && bar.close > upper {
            let std = (upper - lower) / (2.0 * self.std_dev);
            let strength = if std > 0.0 { (bar.close - upper) / std } else { 0.5 };
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                lower, // Use lower band as support reference
            ));
        }

        // Short breakdown (if enabled)
        if !self.long_only && prev_bar.close >= lower && bar.close < lower {
            let std = (upper - lower) / (2.0 * self.std_dev);
            let strength = if std > 0.0 { (lower - bar.close) / std } else { 0.5 };
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
                name: "period".into(),
                param_type: ParamType::Int {
                    min: 10,
                    max: 50,
                    step: 5,
                },
                description: Some("SMA period for middle band".into()),
            },
            ParamDef {
                name: "std_dev".into(),
                param_type: ParamType::Float {
                    min: 1.0,
                    max: 3.5,
                    step: 0.25,
                },
                description: Some("Standard deviation multiplier".into()),
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
    fn test_bands_calculation() {
        let signal_gen = BollingerBreakout::new(5, 2.0, false);
        let prices = vec![100.0, 102.0, 98.0, 101.0, 99.0];
        let bars = make_bars(&prices);

        let (upper, middle, lower) = signal_gen.calculate_bands(&bars, 4).unwrap();

        // Mean = (100+102+98+101+99)/5 = 100
        assert!((middle - 100.0).abs() < 0.01);
        assert!(upper > middle);
        assert!(lower < middle);
    }

    #[test]
    fn test_warmup() {
        let signal_gen = BollingerBreakout::new(10, 2.0, false);
        assert_eq!(signal_gen.warmup_bars(), 10);
    }
}
