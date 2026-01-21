//! Actions for state mutations.
//!
//! Actions are the only way to modify application state.
//! Components dispatch actions, the app applies them centrally.

use trendlab_yolo::leaderboard::LeaderboardType;

/// Application tabs (main panels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// YOLO structural Monte Carlo panel (default)
    #[default]
    Yolo,
    /// Results table panel
    Results,
    /// Equity curve chart panel
    Chart,
    /// Help/documentation panel
    Help,
}

impl Tab {
    /// Get all tabs in display order.
    pub fn all() -> [Tab; 4] {
        [Tab::Yolo, Tab::Results, Tab::Chart, Tab::Help]
    }

    /// Get the next tab (wraps around).
    pub fn next(self) -> Tab {
        match self {
            Tab::Yolo => Tab::Results,
            Tab::Results => Tab::Chart,
            Tab::Chart => Tab::Help,
            Tab::Help => Tab::Yolo,
        }
    }

    /// Get the previous tab (wraps around).
    pub fn prev(self) -> Tab {
        match self {
            Tab::Yolo => Tab::Help,
            Tab::Results => Tab::Yolo,
            Tab::Chart => Tab::Results,
            Tab::Help => Tab::Chart,
        }
    }

    /// Get the 1-indexed number for this tab.
    pub fn number(self) -> u8 {
        match self {
            Tab::Yolo => 1,
            Tab::Results => 2,
            Tab::Chart => 3,
            Tab::Help => 4,
        }
    }

    /// Get tab name for display.
    pub fn name(self) -> &'static str {
        match self {
            Tab::Yolo => "YOLO",
            Tab::Results => "Results",
            Tab::Chart => "Chart",
            Tab::Help => "Help",
        }
    }
}

/// Actions that can be dispatched to modify application state.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // === Application Lifecycle ===
    /// Quit the application
    Quit,

    // === Session Control ===
    /// Start a new YOLO session
    StartSession,
    /// Stop the current session
    StopSession,
    /// Pause or resume the current session
    PauseResume,

    // === Tab Navigation ===
    /// Switch to a specific tab
    SelectTab(Tab),
    /// Move to next tab
    NextTab,
    /// Move to previous tab
    PrevTab,

    // === Navigation ===
    /// Navigate up in a list
    NavigateUp,
    /// Navigate down in a list
    NavigateDown,
    /// Jump to the first item
    NavigateFirst,
    /// Jump to the last item
    NavigateLast,
    /// Page up (half screen)
    PageUp,
    /// Page down (half screen)
    PageDown,

    // === Leaderboard ===
    /// Select a specific leaderboard tab
    SelectLeaderboard(LeaderboardType),
    /// Move to next leaderboard tab
    NextLeaderboard,
    /// Move to previous leaderboard tab
    PrevLeaderboard,
    /// Select the currently highlighted entry
    SelectEntry,

    // === Modals ===
    /// Show help modal
    ShowHelp,
    /// Show export modal
    ShowExport,
    /// Show config modal
    ShowConfig,
    /// Close any open modal
    CloseModal,

    // === Export ===
    /// Export the selected strategy to Pine Script
    Export,

    // === View Options ===
    /// Toggle detail view
    ToggleDetail,
    /// Cycle sort column
    CycleSort,

    /// Tick event (for animations and updates)
    Tick,

    /// No-op action (for unhandled keys)
    None,
}

impl Action {
    /// Check if this action requires a running session.
    pub fn requires_session(&self) -> bool {
        matches!(
            self,
            Action::StopSession
                | Action::PauseResume
                | Action::NavigateUp
                | Action::NavigateDown
                | Action::NavigateFirst
                | Action::NavigateLast
                | Action::PageUp
                | Action::PageDown
                | Action::SelectEntry
                | Action::Export
        )
    }

    /// Check if this is a navigation action.
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            Action::NavigateUp
                | Action::NavigateDown
                | Action::NavigateFirst
                | Action::NavigateLast
                | Action::PageUp
                | Action::PageDown
        )
    }

    /// Check if this action modifies the session.
    pub fn modifies_session(&self) -> bool {
        matches!(
            self,
            Action::StartSession | Action::StopSession | Action::PauseResume
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_session() {
        assert!(!Action::Quit.requires_session());
        assert!(!Action::StartSession.requires_session());
        assert!(Action::StopSession.requires_session());
        assert!(Action::PauseResume.requires_session());
        assert!(Action::NavigateUp.requires_session());
    }

    #[test]
    fn test_is_navigation() {
        assert!(Action::NavigateUp.is_navigation());
        assert!(Action::NavigateDown.is_navigation());
        assert!(!Action::Quit.is_navigation());
        assert!(!Action::StartSession.is_navigation());
    }
}
