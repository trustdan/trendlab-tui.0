//! Exit reference mode for extreme-based exits.
//!
//! # The Stickiness Problem
//!
//! In v1, strategies using rolling references (e.g., 52-week high) for BOTH
//! entry AND exit created "sticky" positions that couldn't exit because the
//! reference kept moving away.
//!
//! # The Solution
//!
//! Every PositionManager that uses price extremes for exit calculations
//! MUST declare its exit reference mode. This makes the behavior explicit
//! and allows the system to detect potentially problematic configurations.

use serde::{Deserialize, Serialize};

/// Exit reference mode for extreme-based exits.
///
/// # Modes
///
/// - `EntryFrozenReference`: Reference is fixed at entry and never updates.
///   Example: "Stop at 10% below the high on the day I entered"
///
/// - `SinceEntryTrailingExtreme`: Reference tracks the extreme since entry.
///   Example: "Stop at 10% below the highest price since I entered"
///   Note: This is different from global rolling high!
///
/// - `SeparateEntryExitLookbacks`: Entry and exit use different windows.
///   Example: "Enter on 200-day high breakout, exit on 50-day low breakdown"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReferenceMode {
    /// Reference fixed at entry, never updates.
    ///
    /// Use when the exit level should be determined once at entry
    /// based on entry conditions.
    EntryFrozenReference,

    /// Tracks extreme since entry (NOT globally).
    ///
    /// Use for trailing stops that follow favorable price movement.
    /// The reference updates as price moves in the trade's favor.
    SinceEntryTrailingExtreme,

    /// Separate lookback windows for entry vs exit.
    ///
    /// Use when entry is based on one indicator (e.g., 200-day breakout)
    /// but exit is based on a different indicator (e.g., 50-day breakdown).
    SeparateEntryExitLookbacks,
}

impl ExitReferenceMode {
    /// Returns true if this mode uses a trailing reference.
    pub fn is_trailing(&self) -> bool {
        matches!(self, ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    /// Returns true if this mode fixes the reference at entry.
    pub fn is_frozen(&self) -> bool {
        matches!(self, ExitReferenceMode::EntryFrozenReference)
    }
}

impl std::fmt::Display for ExitReferenceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitReferenceMode::EntryFrozenReference => write!(f, "Entry Frozen"),
            ExitReferenceMode::SinceEntryTrailingExtreme => write!(f, "Trailing Since Entry"),
            ExitReferenceMode::SeparateEntryExitLookbacks => write!(f, "Separate Lookbacks"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_predicates() {
        assert!(ExitReferenceMode::SinceEntryTrailingExtreme.is_trailing());
        assert!(!ExitReferenceMode::EntryFrozenReference.is_trailing());

        assert!(ExitReferenceMode::EntryFrozenReference.is_frozen());
        assert!(!ExitReferenceMode::SinceEntryTrailingExtreme.is_frozen());
    }

    #[test]
    fn test_display() {
        assert_eq!(
            ExitReferenceMode::SinceEntryTrailingExtreme.to_string(),
            "Trailing Since Entry"
        );
    }
}
