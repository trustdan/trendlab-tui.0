//! Donchian Channel Breakout Signal Generator.
//!
//! Generates long signals when price breaks above the N-period high.
//! Generates short signals when price breaks below the N-period low.

use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::SignalGenerator;
use crate::types::{Bar, Direction, Signal};

/// Donchian Channel Breakout Signal Generator.
///
/// # Strategy
///
/// The Donchian breakout is a classic trend-following entry signal:
/// - Long when price closes above the highest high of the last N bars
/// - Short when price closes below the lowest low of the last N bars
///
/// # Parameters
///
/// - `lookback`: Number of bars for the Donchian channel (default: 20)
/// - `long_only`: Only generate long signals (default: false)
///
/// # Example
///
/// ```ignore
/// let signal_gen = DonchianBreakout::new(20, false);
/// // Will generate long on 20-bar high breakout
/// // Will generate short on 20-bar low breakdown
/// ```
#[derive(Debug, Clone)]
pub struct DonchianBreakout {
    lookback: usize,
    long_only: bool,
}

impl DonchianBreakout {
    /// Create a new Donchian breakout signal generator.
    ///
    /// # Arguments
    /// - `lookback`: Number of bars for the channel
    /// - `long_only`: Only generate long signals
    pub fn new(lookback: usize, long_only: bool) -> Self {
        Self { lookback, long_only }
    }

    /// Create a long-only Donchian breakout.
    pub fn long_only(lookback: usize) -> Self {
        Self::new(lookback, true)
    }
}

impl Default for DonchianBreakout {
    fn default() -> Self {
        Self::new(20, false)
    }
}

impl SignalGenerator for DonchianBreakout {
    fn name(&self) -> &str {
        "DonchianBreakout"
    }

    fn warmup_bars(&self) -> usize {
        self.lookback
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        // Need at least lookback bars of history
        if state.current_idx < self.lookback {
            return None;
        }

        // Get channel levels (excluding current bar)
        let highest = state.highest_high(self.lookback);
        let lowest = state.lowest_low(self.lookback);

        // Long breakout: close above highest high
        if bar.close > highest {
            let raw_strength = if highest > 0.0 {
                (bar.close - highest) / highest
            } else {
                0.0
            };
            return Some(Signal::market(
                Direction::Long,
                raw_strength.clamp(0.0, 1.0),
                highest,
            ));
        }

        // Short breakout: close below lowest low (if enabled)
        if !self.long_only && bar.close < lowest {
            let raw_strength = if lowest > 0.0 {
                (lowest - bar.close) / lowest
            } else {
                0.0
            };
            return Some(Signal::market(
                Direction::Short,
                raw_strength.clamp(0.0, 1.0),
                lowest,
            ));
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "lookback".into(),
                param_type: ParamType::Int {
                    min: 10,
                    max: 252,
                    step: 5,
                },
                description: Some("Donchian channel lookback period".into()),
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

    fn make_state<'a>(bars: &'a [Bar], idx: usize, atr: &'a [f64], adx: &'a [f64]) -> MarketState<'a> {
        MarketState::new(&bars[..=idx], idx, &atr[..=idx], &adx[..=idx])
    }

    #[test]
    fn test_no_signal_during_warmup() {
        let signal_gen = DonchianBreakout::new(20, false);
        let bars = make_bars(&vec![100.0; 25]);
        let atr = vec![1.0; 25];
        let adx = vec![25.0; 25];

        // During warmup (idx < 20), should return None
        for i in 0..20 {
            let state = make_state(&bars, i, &atr, &adx);
            assert!(signal_gen.generate(&bars[i], &state).is_none());
        }
    }

    #[test]
    fn test_long_breakout() {
        let signal_gen = DonchianBreakout::new(5, false);

        // Create bars where last bar breaks above 5-bar high
        let mut prices: Vec<f64> = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0];
        prices.push(105.0); // Breakout bar

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = make_state(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_some());
        let sig = signal.unwrap();
        assert_eq!(sig.direction, Direction::Long);
        // Trigger value should be the prior high (102 + 1.0 = 103.0 for bar at index 2)
    }

    #[test]
    fn test_short_breakout() {
        let signal_gen = DonchianBreakout::new(5, false);

        // Create bars where last bar breaks below 5-bar low
        let mut prices: Vec<f64> = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0];
        prices.push(93.0); // Breakdown bar

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = make_state(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_some());
        let sig = signal.unwrap();
        assert_eq!(sig.direction, Direction::Short);
    }

    #[test]
    fn test_long_only_mode() {
        let signal_gen = DonchianBreakout::long_only(5);

        // Create breakdown scenario
        let mut prices: Vec<f64> = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0];
        prices.push(93.0); // Would be short signal if not long_only

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = make_state(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        // Should NOT generate short signal in long_only mode
        assert!(signal.is_none());
    }

    #[test]
    fn test_no_signal_in_channel() {
        let signal_gen = DonchianBreakout::new(5, false);

        // Price stays within channel
        let prices: Vec<f64> = vec![100.0, 101.0, 102.0, 101.0, 100.0, 99.0, 100.5];

        let bars = make_bars(&prices);
        let atr = vec![1.0; bars.len()];
        let adx = vec![25.0; bars.len()];

        let state = make_state(&bars, 6, &atr, &adx);
        let signal = signal_gen.generate(&bars[6], &state);

        assert!(signal.is_none());
    }

    #[test]
    fn test_parameter_spec() {
        let signal_gen = DonchianBreakout::new(20, false);
        let params = signal_gen.parameter_spec();

        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "lookback");
        assert_eq!(params[1].name, "long_only");
    }

    #[test]
    fn test_warmup_bars() {
        let signal_gen = DonchianBreakout::new(50, false);
        assert_eq!(signal_gen.warmup_bars(), 50);
    }

    #[test]
    fn test_box_clone() {
        let signal_gen = DonchianBreakout::new(20, true);
        let cloned = signal_gen.box_clone();

        assert_eq!(cloned.name(), "DonchianBreakout");
        assert_eq!(cloned.warmup_bars(), 20);
    }
}
