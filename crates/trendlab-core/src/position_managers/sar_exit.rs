//! Parabolic SAR Exit Position Manager.
//!
//! Uses Parabolic SAR as a dynamic trailing stop.

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// Parabolic SAR Exit Position Manager.
///
/// # Strategy
///
/// Uses the Parabolic SAR (Stop And Reverse) indicator as a trailing stop:
/// - SAR accelerates towards price as the trend continues
/// - Exit when price crosses the SAR
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - SAR calculation starts fresh from entry.
///
/// # Parameters
///
/// - `af_start`: Initial acceleration factor
/// - `af_max`: Maximum acceleration factor
#[derive(Debug, Clone)]
pub struct SarExit {
    af_start: f64,
    af_max: f64,
    // Internal state
    sar: Option<f64>,
    ep: f64, // Extreme point
    af: f64, // Current acceleration factor
    direction: Option<Direction>,
}

impl SarExit {
    /// Create a new Parabolic SAR exit position manager.
    pub fn new(af_start: f64, af_max: f64) -> Self {
        Self {
            af_start,
            af_max,
            sar: None,
            ep: 0.0,
            af: af_start,
            direction: None,
        }
    }
}

impl Default for SarExit {
    fn default() -> Self {
        Self::new(0.02, 0.20)
    }
}

impl PositionManager for SarExit {
    fn name(&self) -> &str {
        "SarExit"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        self.direction = Some(signal.direction);
        self.af = self.af_start;

        // Initialize SAR based on direction
        match signal.direction {
            Direction::Long => {
                // SAR starts below price
                self.sar = Some(entry_bar.low);
                self.ep = entry_price;
            }
            Direction::Short => {
                // SAR starts above price
                self.sar = Some(entry_bar.high);
                self.ep = entry_price;
            }
        }
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, _state: &MarketState) -> Action {
        let current_sar = match self.sar {
            Some(s) => s,
            None => return Action::Hold,
        };

        let direction = self.direction.unwrap_or(position.direction);

        // Check if SAR is hit
        let sar_hit = match direction {
            Direction::Long => bar.low <= current_sar,
            Direction::Short => bar.high >= current_sar,
        };

        if sar_hit {
            return Action::Exit(ExitReason::StopHit);
        }

        // Update extreme point and acceleration factor
        match direction {
            Direction::Long => {
                if bar.high > self.ep {
                    self.ep = bar.high;
                    self.af = (self.af + self.af_start).min(self.af_max);
                }
            }
            Direction::Short => {
                if bar.low < self.ep {
                    self.ep = bar.low;
                    self.af = (self.af + self.af_start).min(self.af_max);
                }
            }
        }

        // Calculate new SAR
        let new_sar = current_sar + self.af * (self.ep - current_sar);

        // Ensure SAR doesn't cross into recent bars
        let clamped_sar = match direction {
            Direction::Long => new_sar.min(bar.low).min(position.low_since_entry),
            Direction::Short => new_sar.max(bar.high).max(position.high_since_entry),
        };

        self.sar = Some(clamped_sar);

        Action::AdjustStop(clamped_sar)
    }

    fn stop_price(&self) -> Option<f64> {
        self.sar
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "af_start".into(),
                param_type: ParamType::Float {
                    min: 0.01,
                    max: 0.05,
                    step: 0.01,
                },
                description: Some("Initial acceleration factor".into()),
            },
            ParamDef {
                name: "af_max".into(),
                param_type: ParamType::Float {
                    min: 0.10,
                    max: 0.30,
                    step: 0.05,
                },
                description: Some("Maximum acceleration factor".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(Self::new(self.af_start, self.af_max))
    }

    fn reset(&mut self) {
        self.sar = None;
        self.ep = 0.0;
        self.af = self.af_start;
        self.direction = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let pm = SarExit::default();
        assert_eq!(pm.name(), "SarExit");
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = SarExit::default();
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }

    #[test]
    fn test_default_params() {
        let pm = SarExit::default();
        assert!((pm.af_start - 0.02).abs() < 0.001);
        assert!((pm.af_max - 0.20).abs() < 0.001);
    }
}
