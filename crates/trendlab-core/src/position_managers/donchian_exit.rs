//! Donchian Channel Exit Position Manager.
//!
//! Exits when price breaks the opposite Donchian channel.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Donchian Channel Exit Position Manager.
///
/// # Strategy
///
/// Classic Turtle exit rule: exit on N-bar low (longs) or N-bar high (shorts):
/// - Long exit: price closes below the N-bar low
/// - Short exit: price closes above the N-bar high
///
/// Uses a separate (typically shorter) lookback than entry to capture profits.
///
/// # Exit Reference Mode
///
/// Uses `SeparateEntryExitLookbacks` - distinct windows for entry vs exit.
///
/// # Parameters
///
/// - `exit_lookback`: Number of bars for exit channel (typically 10 vs 20 entry)
#[derive(Debug, Clone)]
pub struct DonchianExit {
    exit_lookback: usize,
    // Internal state
    stop_price: Option<f64>,
}

impl DonchianExit {
    /// Create a new Donchian channel exit position manager.
    pub fn new(exit_lookback: usize) -> Self {
        Self {
            exit_lookback,
            stop_price: None,
        }
    }
}

impl Default for DonchianExit {
    fn default() -> Self {
        Self::new(10) // 10-bar exit (vs 20-bar entry)
    }
}

impl PositionManager for DonchianExit {
    fn name(&self) -> &str {
        "DonchianExit"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SeparateEntryExitLookbacks)
    }

    fn on_entry(&mut self, _entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        // Set initial stop as a simple buffer
        // Real stop will be calculated from Donchian channel
        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price * 0.90, // 10% buffer
            Direction::Short => entry_price * 1.10,
        });
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
        if state.current_idx < self.exit_lookback {
            return Action::Hold;
        }

        // Calculate exit levels from Donchian channel
        let exit_low = state.lowest_low(self.exit_lookback);
        let exit_high = state.highest_high(self.exit_lookback);

        // Check exit condition
        let should_exit = match position.direction {
            Direction::Long => bar.close < exit_low,
            Direction::Short => bar.close > exit_high,
        };

        if should_exit {
            return Action::Exit(ExitReason::SignalExit);
        }

        // Update stop to channel level
        let new_stop = match position.direction {
            Direction::Long => exit_low,
            Direction::Short => exit_high,
        };

        // Ratchet stop in favorable direction only
        let should_update = match position.direction {
            Direction::Long => self.stop_price.map(|s| new_stop > s).unwrap_or(true),
            Direction::Short => self.stop_price.map(|s| new_stop < s).unwrap_or(true),
        };

        if should_update {
            self.stop_price = Some(new_stop);
            return Action::AdjustStop(new_stop);
        }

        Action::Hold
    }

    fn stop_price(&self) -> Option<f64> {
        self.stop_price
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![ParamDef {
            name: "exit_lookback".into(),
            param_type: ParamType::Int {
                min: 5,
                max: 30,
                step: 5,
            },
            description: Some("Exit channel lookback".into()),
        }]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.exit_lookback))
    }

    fn reset(&mut self) {
        self.stop_price = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let pm = DonchianExit::default();
        assert_eq!(pm.name(), "DonchianExit");
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = DonchianExit::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SeparateEntryExitLookbacks)
        );
    }
}
