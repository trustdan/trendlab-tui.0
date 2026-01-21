//! Signal filter trait.
//!
//! SignalFilters gate entry signals based on regime conditions (ADX, volatility, etc.)
//! and can force exit of existing positions when conditions deteriorate.

use crate::market_state::MarketState;
use crate::param::ParamDef;
use crate::types::{Bar, Position, Signal};

/// Signal filter component.
///
/// # Contract
///
/// - MUST return boolean decisions only
/// - MUST NOT modify signals or positions
/// - MAY force exit of existing positions (regime change)
///
/// # Use Cases
///
/// - ADX trend filter: Only allow signals when ADX > threshold
/// - Volatility filter: Suppress signals in high/low volatility
/// - Seasonal filter: Avoid certain months
/// - Correlation filter: Reduce signals when correlation high
///
/// # Example
/// ```ignore
/// struct AdxFilter {
///     min_adx: f64,
///     force_exit_below: f64,
/// }
///
/// impl SignalFilter for AdxFilter {
///     fn allow_signal(&self, _signal: &Signal, _bar: &Bar, state: &MarketState) -> bool {
///         state.current_adx() >= self.min_adx
///     }
///
///     fn force_exit(&self, _position: &Position, _bar: &Bar, state: &MarketState) -> bool {
///         state.current_adx() < self.force_exit_below
///     }
/// }
/// ```
pub trait SignalFilter: Send + Sync {
    /// Unique identifier for logging.
    fn name(&self) -> &str;

    /// Whether to allow this entry signal.
    ///
    /// # Returns
    /// - `true`: Signal passes, can proceed to execution
    /// - `false`: Signal suppressed, no entry
    fn allow_signal(&self, signal: &Signal, bar: &Bar, state: &MarketState) -> bool;

    /// Whether to force exit of existing position.
    ///
    /// # Use Case
    /// Regime change (e.g., ADX drops below threshold during trade)
    ///
    /// # Returns
    /// - `true`: Force immediate exit
    /// - `false`: Continue holding
    fn force_exit(&self, position: &Position, bar: &Bar, state: &MarketState) -> bool;

    /// Parameter specification for Monte Carlo sampling.
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn SignalFilter>;
}

impl Clone for Box<dyn SignalFilter> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// A pass-through filter that allows all signals and never forces exit.
#[derive(Debug, Clone)]
pub struct NoFilter;

impl SignalFilter for NoFilter {
    fn name(&self) -> &str {
        "NoFilter"
    }

    fn allow_signal(&self, _signal: &Signal, _bar: &Bar, _state: &MarketState) -> bool {
        true
    }

    fn force_exit(&self, _position: &Position, _bar: &Bar, _state: &MarketState) -> bool {
        false
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![]
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

    #[test]
    fn test_no_filter() {
        let filter = NoFilter;
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let bar = Bar::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            100.0,
            105.0,
            95.0,
            102.0,
            1_000_000,
            0,
        );
        let atr = [1.0];
        let adx = [25.0];
        let bars = [bar.clone()];
        let state = MarketState::new(&bars, 0, &atr, &adx);

        assert!(filter.allow_signal(&signal, &bar, &state));

        let position = Position::new(
            0,
            bar.date,
            100.0,
            Direction::Long,
            10000.0,
            signal,
        );
        assert!(!filter.force_exit(&position, &bar, &state));
    }
}
