//! Vim-native keybinding system.
//!
//! Keybindings are context-sensitive:
//! - Global keys work everywhere
//! - Modal keys work only when a modal is open
//! - List keys work in list contexts

use crate::action::{Action, Tab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Keybinding context determines which keys are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyContext {
    /// No modal open, normal operation
    Normal,
    /// Help modal is open
    HelpModal,
    /// Export modal is open
    ExportModal,
    /// Config modal is open
    ConfigModal,
}

/// Convert a key event to an action based on context.
pub fn key_to_action(key: KeyEvent, context: KeyContext) -> Action {
    // Modal-specific handling first
    match context {
        KeyContext::HelpModal | KeyContext::ExportModal | KeyContext::ConfigModal => {
            return modal_key_to_action(key, context);
        }
        KeyContext::Normal => {}
    }

    // Global keys (work in normal context)
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Esc, _) => Action::Quit,

        // Session control
        (KeyCode::Enter, KeyModifiers::NONE) => Action::StartSession,
        (KeyCode::Char(' '), KeyModifiers::NONE) => Action::PauseResume,
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Action::StopSession,

        // Vim navigation
        (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => Action::NavigateDown,
        (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => Action::NavigateUp,
        (KeyCode::Char('g'), KeyModifiers::NONE) => Action::NavigateFirst, // gg handled specially
        (KeyCode::Char('G'), KeyModifiers::SHIFT) | (KeyCode::Char('G'), KeyModifiers::NONE) => {
            Action::NavigateLast
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::PageDown,
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Action::PageUp,

        // Tab navigation (main panels)
        (KeyCode::Char('1'), KeyModifiers::NONE) => Action::SelectTab(Tab::Yolo),
        (KeyCode::Char('2'), KeyModifiers::NONE) => Action::SelectTab(Tab::Results),
        (KeyCode::Char('3'), KeyModifiers::NONE) => Action::SelectTab(Tab::Chart),
        (KeyCode::Char('4'), KeyModifiers::NONE) => Action::SelectTab(Tab::Help),
        (KeyCode::Tab, KeyModifiers::NONE) => Action::NextTab,
        (KeyCode::BackTab, KeyModifiers::SHIFT) | (KeyCode::BackTab, KeyModifiers::NONE) => Action::PrevTab,

        // Leaderboard navigation (within YOLO tab)
        (KeyCode::Char('h'), KeyModifiers::NONE) | (KeyCode::Left, _) => Action::PrevLeaderboard,
        (KeyCode::Char('l'), KeyModifiers::NONE) | (KeyCode::Right, _) => Action::NextLeaderboard,

        // Actions
        (KeyCode::Char('e'), KeyModifiers::NONE) => Action::ShowExport,
        (KeyCode::Char('?'), KeyModifiers::NONE)
        | (KeyCode::Char('/'), KeyModifiers::SHIFT)
        | (KeyCode::F(1), _) => Action::ShowHelp,
        (KeyCode::Char('c'), KeyModifiers::NONE) => Action::ShowConfig,

        // View options
        (KeyCode::Char('d'), KeyModifiers::NONE) => Action::ToggleDetail,
        (KeyCode::Char('o'), KeyModifiers::NONE) => Action::CycleSort,

        _ => Action::None,
    }
}

/// Handle keys when a modal is open.
fn modal_key_to_action(key: KeyEvent, context: KeyContext) -> Action {
    match key.code {
        // Close modal
        KeyCode::Esc | KeyCode::Char('q') => Action::CloseModal,
        KeyCode::Char('?') if context == KeyContext::HelpModal => Action::CloseModal,

        // Navigation within modal
        KeyCode::Char('j') | KeyCode::Down => Action::NavigateDown,
        KeyCode::Char('k') | KeyCode::Up => Action::NavigateUp,
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => Action::PageDown,
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => Action::PageUp,

        // Confirm in export modal
        KeyCode::Enter if context == KeyContext::ExportModal => Action::Export,

        _ => Action::None,
    }
}

/// Get a description of a keybinding for display in help.
pub fn key_description(key: KeyCode, modifiers: KeyModifiers) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }

    let key_str = match key {
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => "?".to_string(),
    };

    parts.push(&key_str);
    if parts.len() > 1 {
        parts.join("+")
    } else {
        key_str
    }
}

/// All keybindings for display in help panel.
pub fn all_keybindings() -> Vec<(&'static str, &'static str)> {
    vec![
        // Session control
        ("Enter", "Start YOLO session"),
        ("Space", "Pause/Resume session"),
        ("Ctrl+s", "Stop session"),
        // Tab navigation
        ("1-4", "Switch tab (YOLO/Results/Chart/Help)"),
        ("Tab", "Next tab"),
        ("Shift+Tab", "Previous tab"),
        // List navigation
        ("j / Down", "Move down"),
        ("k / Up", "Move up"),
        ("g", "Jump to first"),
        ("G", "Jump to last"),
        ("Ctrl+d", "Page down"),
        ("Ctrl+u", "Page up"),
        // Leaderboard (in YOLO tab)
        ("h / Left", "Previous leaderboard"),
        ("l / Right", "Next leaderboard"),
        // Actions
        ("e", "Export to Pine"),
        ("d", "Toggle detail"),
        ("o", "Cycle sort"),
        // Modals
        ("?", "Show help"),
        ("c", "Show config"),
        // Quit
        ("q / Esc", "Quit"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quit_keys() {
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(key_to_action(q, KeyContext::Normal), Action::Quit);

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(key_to_action(esc, KeyContext::Normal), Action::Quit);
    }

    #[test]
    fn test_navigation_keys() {
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(key_to_action(j, KeyContext::Normal), Action::NavigateDown);

        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(key_to_action(k, KeyContext::Normal), Action::NavigateUp);
    }

    #[test]
    fn test_leaderboard_tabs() {
        let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(key_to_action(h, KeyContext::Normal), Action::PrevLeaderboard);

        let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(key_to_action(l, KeyContext::Normal), Action::NextLeaderboard);
    }

    #[test]
    fn test_modal_close() {
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            key_to_action(esc, KeyContext::HelpModal),
            Action::CloseModal
        );
    }

    #[test]
    fn test_key_description() {
        assert_eq!(
            key_description(KeyCode::Char('d'), KeyModifiers::CONTROL),
            "Ctrl+d"
        );
        assert_eq!(
            key_description(KeyCode::Char('q'), KeyModifiers::NONE),
            "q"
        );
        assert_eq!(
            key_description(KeyCode::Enter, KeyModifiers::NONE),
            "Enter"
        );
    }
}
