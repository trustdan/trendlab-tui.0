//! Rate of Change (ROC) Signal Generator.
//!
//! Generates signals based on price momentum using rate of change.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Rate of Change (ROC) Signal Generator.
///
/// # Strategy
///
/// ROC measures momentum by comparing current price to price N periods ago:
/// - Long when ROC crosses above threshold
/// - Short when ROC crosses below negative threshold
///
/// # Formula
///
/// ROC = ((Close - Close_n) / Close_n) * 100
///
/// # Parameters
///
/// - `period`: Lookback period for ROC calculation (default: 14)
/// - `threshold`: Threshold for signal generation (default: 0.0)
/// - `long_only`: Only generate long signals (default: false)
#[derive(Debug, Clone)]
pub struct RocMomentum {
    period: usize,
    threshold: f64,
    long_only: bool,
}

impl RocMomentum {
    /// Create a new ROC momentum signal generator.
    pub fn new(period: usize, threshold: f64, long_only: bool) -> Self {
        Self {
            period,
            threshold,
            long_only,
        }
    }

    /// Calculate ROC value.
    fn calculate_roc(&self, bars: &[Bar], end_idx: usize) -> Option<f64> {
        if end_idx < self.period {
            return None;
        }

        let current_close = bars[end_idx].close;
        let past_close = bars[end_idx - self.period].close;

        if past_close <= 0.0 {
            return None;
        }

        Some(((current_close - past_close) / past_close) * 100.0)
    }
}

impl Default for RocMomentum {
    fn default() -> Self {
        Self::new(14, 0.0, false)
    }
}

impl SignalGenerator for RocMomentum {
    fn name(&self) -> &str {
        "RocMomentum"
    }

    fn warmup_bars(&self) -> usize {
        self.period + 1
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.period + 1 {
            return None;
        }

        let roc_now = self.calculate_roc(state.bars, state.current_idx)?;
        let roc_prev = self.calculate_roc(state.bars, state.current_idx - 1)?;

        // Long signal: ROC crosses above threshold
        if roc_prev <= self.threshold && roc_now > self.threshold {
            let strength = (roc_now / 10.0).min(1.0); // Normalize to 0-1
            return Some(Signal::market(
                Direction::Long,
                strength.clamp(0.0, 1.0),
                bar.close,
            ));
        }

        // Short signal (if enabled): ROC crosses below negative threshold
        if !self.long_only && roc_prev >= -self.threshold && roc_now < -self.threshold {
            let strength = (roc_now.abs() / 10.0).min(1.0);
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
                    min: 5,
                    max: 50,
                    step: 5,
                },
                description: Some("ROC lookback period".into()),
            },
            ParamDef {
                name: "threshold".into(),
                param_type: ParamType::Float {
                    min: 0.0,
                    max: 10.0,
                    step: 1.0,
                },
                description: Some("Signal threshold (%)".into()),
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
    fn test_roc_calculation() {
        let signal_gen = RocMomentum::new(5, 0.0, false);
        let prices = vec![100.0, 100.0, 100.0, 100.0, 100.0, 110.0]; // 10% increase
        let bars = make_bars(&prices);

        let roc = signal_gen.calculate_roc(&bars, 5).unwrap();
        assert!((roc - 10.0).abs() < 0.01); // 10% ROC
    }

    #[test]
    fn test_warmup() {
        let signal_gen = RocMomentum::new(14, 0.0, false);
        assert_eq!(signal_gen.warmup_bars(), 15);
    }
}
