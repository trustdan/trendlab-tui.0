//! Signal generator trait.
//!
//! SignalGenerators are responsible for identifying entry opportunities.
//! They produce signals with intent and levels but NO exit logic.

use crate::market_state::MarketState;
use crate::param::ParamDef;
use crate::types::{Bar, Signal};

/// Signal generator component.
///
/// # Contract
///
/// - MUST produce entry signals only (no exit logic)
/// - MUST NOT track position state or know about current holdings
/// - MUST NOT peek beyond bar N when processing bar N
/// - SHOULD return None during warmup period
///
/// # State
///
/// SignalGenerators may maintain indicator state (e.g., moving average values)
/// but MUST NOT maintain position state. State is per-indicator, not per-trade.
///
/// # Example
/// ```ignore
/// struct DonchianBreakout {
///     lookback: usize,
/// }
///
/// impl SignalGenerator for DonchianBreakout {
///     fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
///         if state.current_idx < self.lookback {
///             return None; // Warmup
///         }
///
///         let high = state.highest_high(self.lookback);
///         if bar.high > high {
///             return Some(Signal::market(Direction::Long, 1.0, high));
///         }
///         None
///     }
/// }
/// ```
pub trait SignalGenerator: Send + Sync {
    /// Unique identifier for logging and leaderboards.
    fn name(&self) -> &str;

    /// Minimum bars required before signals are valid.
    fn warmup_bars(&self) -> usize;

    /// Generate an entry signal for the current bar.
    ///
    /// # Arguments
    /// - `bar`: The current bar being processed
    /// - `state`: Market state up to and including current bar
    ///
    /// # Returns
    /// - `Some(Signal)` if entry conditions are met
    /// - `None` if no entry signal
    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal>;

    /// Parameter specification for Monte Carlo sampling.
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn SignalGenerator>;
}

impl Clone for Box<dyn SignalGenerator> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[cfg(test)]
mod tests {
    // Trait tests would go here once we have implementations
}
