//! Aroon Crossover Signal Generator.
//!
//! Generates signals based on Aroon indicator crossovers.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Aroon Crossover Signal Generator.
///
/// # Strategy
///
/// The Aroon indicator measures time since last high/low:
/// - Long when Aroon Up crosses above Aroon Down
/// - Short when Aroon Down crosses above Aroon Up
///
/// # Formula
///
/// Aroon Up = ((period - bars_since_high) / period) * 100
/// Aroon Down = ((period - bars_since_low) / period) * 100
///
/// # Parameters
///
/// - `period`: Lookback period (default: 25)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct AroonCrossover {
    period: usize,
    long_only: bool,
}

impl AroonCrossover {
    /// Create a new Aroon crossover signal generator.
    pub fn new(period: usize, long_only: bool) -> Self {
        Self { period, long_only }
    }

    /// Calculate Aroon Up and Down values.
    /// Returns (aroon_up, aroon_down).
    fn calculate_aroon(&self, bars: &[Bar], end_idx: usize) -> Option<(f64, f64)> {
        if end_idx < self.period {
            return None;
        }

        let start = end_idx - self.period;
        let window = &bars[start..=end_idx];

        // Find bars since highest high and lowest low
        let (mut highest_idx, mut highest_val) = (0, f64::MIN);
        let (mut lowest_idx, mut lowest_val) = (0, f64::MAX);

        for (i, bar) in window.iter().enumerate() {
            if bar.high >= highest_val {
                highest_val = bar.high;
                highest_idx = i;
            }
            if bar.low <= lowest_val {
                lowest_val = bar.low;
                lowest_idx = i;
            }
        }

        let bars_since_high = self.period - highest_idx;
        let bars_since_low = self.period - lowest_idx;

        let aroon_up = ((self.period - bars_since_high) as f64 / self.period as f64) * 100.0;
        let aroon_down = ((self.period - bars_since_low) as f64 / self.period as f64) * 100.0;

        Some((aroon_up, aroon_down))
    }
}

impl Default for AroonCrossover {
    fn default() -> Self {
        Self::new(25, false)
    }
}

impl SignalGenerator for AroonCrossover {
    fn name(&self) -> &str {
        "AroonCrossover"
    }

    fn warmup_bars(&self) -> usize {
        self.period + 1
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.period + 1 {
            return None;
        }

        let (up_now, down_now) = self.calculate_aroon(state.bars, state.current_idx)?;
        let (up_prev, down_prev) = self.calculate_aroon(state.bars, state.current_idx - 1)?;

        // Bullish crossover: Aroon Up crosses above Aroon Down
        if up_prev <= down_prev && up_now > down_now {
            let strength = (up_now - down_now) / 100.0;
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                bar.close,
            ));
        }

        // Bearish crossover (if enabled)
        if !self.long_only && down_prev <= up_prev && down_now > up_now {
            let strength = (down_now - up_now) / 100.0;
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
                name: "period".into(),
                param_type: ParamType::Int {
                    min: 10,
                    max: 50,
                    step: 5,
                },
                description: Some("Aroon lookback period".into()),
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
    fn test_aroon_calculation() {
        let signal_gen = AroonCrossover::new(5, false);
        // Uptrend: prices increasing, so high was most recent
        let prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        let bars = make_bars(&prices);

        let (aroon_up, _aroon_down) = signal_gen.calculate_aroon(&bars, 5).unwrap();

        // High is at the end (most recent), so aroon_up should be 100
        assert!((aroon_up - 100.0).abs() < 0.01);
        // Low is also at the end, so aroon_down should also be high
        assert!(aroon_up >= 80.0);
    }

    #[test]
    fn test_warmup() {
        let signal_gen = AroonCrossover::new(25, false);
        assert_eq!(signal_gen.warmup_bars(), 26);
    }
}
