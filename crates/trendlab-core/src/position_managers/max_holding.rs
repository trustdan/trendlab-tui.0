//! Max Holding Period Position Manager.
//!
//! Exits after a maximum number of bars regardless of price action.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, ExitReason, Position, Signal};

/// Max Holding Period Position Manager.
///
/// # Strategy
///
/// Simple time-based exit that closes the position after N bars.
/// Can be combined with other managers for time limits on trades.
///
/// # Exit Reference Mode
///
/// Returns `None` - this manager doesn't use price extremes.
///
/// # Parameters
///
/// - `max_bars`: Maximum number of bars to hold a position
#[derive(Debug, Clone)]
pub struct MaxHoldingPeriod {
    max_bars: usize,
    // Internal state
    entry_bar_idx: usize,
}

impl MaxHoldingPeriod {
    /// Create a new max holding period position manager.
    pub fn new(max_bars: usize) -> Self {
        Self {
            max_bars,
            entry_bar_idx: 0,
        }
    }
}

impl Default for MaxHoldingPeriod {
    fn default() -> Self {
        Self::new(20) // 20 bars max
    }
}

impl PositionManager for MaxHoldingPeriod {
    fn name(&self) -> &str {
        "MaxHoldingPeriod"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        None // No price-based exit reference
    }

    fn on_entry(&mut self, entry_bar: &Bar, _entry_price: f64, _signal: &Signal) {
        self.entry_bar_idx = entry_bar.idx;
    }

    fn on_bar(&mut self, bar: &Bar, _position: &Position, _state: &MarketState) -> Action {
        let bars_held = bar.idx.saturating_sub(self.entry_bar_idx);

        if bars_held >= self.max_bars {
            return Action::Exit(ExitReason::TimeExit);
        }

        Action::Hold
    }

    fn stop_price(&self) -> Option<f64> {
        None // No stop price for time-based exit
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![ParamDef {
            name: "max_bars".into(),
            param_type: ParamType::Int {
                min: 5,
                max: 252,
                step: 5,
            },
            description: Some("Maximum bars to hold position".into()),
        }]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.max_bars))
    }

    fn reset(&mut self) {
        self.entry_bar_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_reference_mode() {
        let pm = MaxHoldingPeriod::default();
        assert!(pm.exit_reference_mode().is_none());
    }

    #[test]
    fn test_name() {
        let pm = MaxHoldingPeriod::default();
        assert_eq!(pm.name(), "MaxHoldingPeriod");
    }
}
