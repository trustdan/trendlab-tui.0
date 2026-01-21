//! Help panel and modal rendering.
//!
//! Displays all keybindings organized by category.
//! Can be rendered as either a modal overlay or a full panel (Tab 4).

use crate::app::App;
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render the help modal (overlay).
pub fn render(frame: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::border_focused()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = build_help_content();
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render the help panel (full tab).
pub fn render_panel(frame: &mut Frame, _app: &App, area: Rect) {
    let block = panel_block("Help", true);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = build_help_content();
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Build the help content lines.
fn build_help_content() -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        "TrendLab v2 - Keybindings",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Tab Navigation
    lines.push(Line::from(Span::styled(
        "Tab Navigation",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("1", "YOLO tab - Monte Carlo research"),
        ("2", "Results tab - Strategy table"),
        ("3", "Chart tab - Equity curves"),
        ("4", "Help tab (this screen)"),
        ("Tab", "Next tab"),
        ("Shift+Tab", "Previous tab"),
    ]);
    lines.push(Line::from(""));

    // Session Control
    lines.push(Line::from(Span::styled(
        "Session Control",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("Enter", "Start YOLO session"),
        ("Space", "Pause/Resume session"),
        ("Ctrl+s", "Stop session"),
    ]);
    lines.push(Line::from(""));

    // Navigation
    lines.push(Line::from(Span::styled(
        "List Navigation",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("j / Down", "Move down"),
        ("k / Up", "Move up"),
        ("g", "Jump to first"),
        ("G", "Jump to last"),
        ("Ctrl+d", "Page down"),
        ("Ctrl+u", "Page up"),
    ]);
    lines.push(Line::from(""));

    // Leaderboard (YOLO tab)
    lines.push(Line::from(Span::styled(
        "Leaderboard (YOLO tab)",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("h / Left", "Previous leaderboard"),
        ("l / Right", "Next leaderboard"),
    ]);
    lines.push(Line::from(""));

    // Actions
    lines.push(Line::from(Span::styled(
        "Actions",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("e", "Export to Pine Script"),
        ("c", "Configuration"),
    ]);
    lines.push(Line::from(""));

    // General
    lines.push(Line::from(Span::styled(
        "General",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    add_keybindings(&mut lines, &[
        ("?", "Toggle help modal"),
        ("q / Esc", "Quit"),
    ]);

    lines
}

/// Add keybindings to the help content.
fn add_keybindings(lines: &mut Vec<Line<'static>>, bindings: &[(&'static str, &'static str)]) {
    for (key, description) in bindings {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:12}", key),
                Style::default().fg(Colors::help_key()),
            ),
            Span::styled(*description, Style::default().fg(Colors::help_body())),
        ]));
    }
}
