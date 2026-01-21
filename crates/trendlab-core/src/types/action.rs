//! Position manager action types.

use serde::{Deserialize, Serialize};

/// Reason for exiting a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    /// Stop loss was hit
    StopHit,

    /// Take profit target reached
    TakeProfit,

    /// Signal-based exit (e.g., opposite signal)
    SignalExit,

    /// Filter forced exit (regime change)
    FilterForceExit,

    /// Time-based exit (held too long)
    TimeExit,

    /// End of data reached
    EndOfData,
}

impl std::fmt::Display for ExitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitReason::StopHit => write!(f, "Stop Hit"),
            ExitReason::TakeProfit => write!(f, "Take Profit"),
            ExitReason::SignalExit => write!(f, "Signal Exit"),
            ExitReason::FilterForceExit => write!(f, "Filter Exit"),
            ExitReason::TimeExit => write!(f, "Time Exit"),
            ExitReason::EndOfData => write!(f, "End of Data"),
        }
    }
}

/// Action returned by PositionManager on each bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Continue holding the position
    Hold,

    /// Adjust the stop price
    AdjustStop(f64),

    /// Scale out of the position by percentage
    ScaleOut {
        /// Percentage to exit (0.0 to 1.0)
        percent: f64,
        /// Reason for partial exit
        reason: ExitReason,
    },

    /// Exit the entire position
    Exit(ExitReason),
}

impl Action {
    /// Returns true if this action results in any exit.
    #[inline]
    pub fn is_exit(&self) -> bool {
        matches!(self, Action::Exit(_) | Action::ScaleOut { .. })
    }

    /// Returns true if this is a full exit.
    #[inline]
    pub fn is_full_exit(&self) -> bool {
        matches!(self, Action::Exit(_))
    }

    /// Returns the exit reason if this is an exit action.
    pub fn exit_reason(&self) -> Option<ExitReason> {
        match self {
            Action::Exit(reason) => Some(*reason),
            Action::ScaleOut { reason, .. } => Some(*reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_is_exit() {
        assert!(!Action::Hold.is_exit());
        assert!(!Action::AdjustStop(95.0).is_exit());
        assert!(Action::Exit(ExitReason::StopHit).is_exit());
        assert!(Action::ScaleOut {
            percent: 0.5,
            reason: ExitReason::TakeProfit
        }
        .is_exit());
    }

    #[test]
    fn test_exit_reason_display() {
        assert_eq!(ExitReason::StopHit.to_string(), "Stop Hit");
        assert_eq!(ExitReason::TakeProfit.to_string(), "Take Profit");
    }
}
