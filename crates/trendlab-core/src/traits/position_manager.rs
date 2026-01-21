//! Position manager trait.
//!
//! PositionManagers handle open trades: trailing stops, targets, time exits, etc.
//! They MUST declare their exit reference mode and initialize state at entry.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::ParamDef;
use crate::types::{Action, Bar, Position, Signal};

/// Position manager component.
///
/// # Contract
///
/// - MUST initialize state at entry (via `on_entry`)
/// - MUST declare exit reference mode for extreme-based exits
/// - MUST NOT access SignalGenerator's internal state
/// - MUST output actions only (Hold, AdjustStop, Exit), not fills
///
/// # State Initialization
///
/// Critical: State like `high_since_entry` starts from the ENTRY bar,
/// not from historical data. This prevents the stickiness problem.
///
/// # Example
/// ```ignore
/// struct AtrTrailingStop {
///     multiplier: f64,
///     stop_price: Option<f64>,
///     high_since_entry: f64,
/// }
///
/// impl PositionManager for AtrTrailingStop {
///     fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, _signal: &Signal) {
///         // Initialize from entry, NOT from historical high!
///         self.high_since_entry = entry_price;
///         self.stop_price = Some(entry_price - self.multiplier * entry_bar.atr);
///     }
///
///     fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
///         // Update trailing high
///         self.high_since_entry = self.high_since_entry.max(bar.high);
///
///         // Calculate new stop
///         let new_stop = self.high_since_entry - self.multiplier * state.current_atr();
///
///         if new_stop > self.stop_price.unwrap_or(0.0) {
///             self.stop_price = Some(new_stop);
///             return Action::AdjustStop(new_stop);
///         }
///
///         Action::Hold
///     }
/// }
/// ```
pub trait PositionManager: Send + Sync {
    /// Unique identifier for logging and leaderboards.
    fn name(&self) -> &str;

    /// Exit reference mode (required for extreme-based exits).
    ///
    /// Return `None` only if this PM doesn't use price extremes for exits.
    fn exit_reference_mode(&self) -> Option<ExitReferenceMode>;

    /// Initialize state when a new position is opened.
    ///
    /// # Important
    /// This is called AFTER the fill occurs. State should be
    /// initialized from the entry context, not historical data.
    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal);

    /// Process a bar and return the management action.
    ///
    /// # Arguments
    /// - `bar`: Current bar
    /// - `position`: Current position state (maintained by engine)
    /// - `state`: Market state
    ///
    /// # Returns
    /// Action to take: Hold, AdjustStop, ScaleOut, or Exit
    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action;

    /// Current stop price (for logging and execution model).
    fn stop_price(&self) -> Option<f64>;

    /// Parameter specification for Monte Carlo sampling.
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object with FRESH state.
    ///
    /// # Important
    /// The returned clone MUST have fresh state (no position-specific data).
    /// This is called at the start of each backtest run.
    fn box_clone(&self) -> Box<dyn PositionManager>;

    /// Reset state for a new run (called by engine).
    fn reset(&mut self);
}

impl Clone for Box<dyn PositionManager> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[cfg(test)]
mod tests {
    // Trait tests would go here once we have implementations
}
