//! Status bar rendering.
//!
//! Shows context-sensitive keybinding hints at the bottom of the screen.

use crate::app::{App, AppPhase};
use crate::colors::Colors;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render the status bar.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Check for status/error messages first
    if let Some(ref error) = app.error_message {
        let line = Line::from(Span::styled(
            error.clone(),
            Style::default()
                .fg(Colors::danger())
                .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    if let Some(ref status) = app.status_message {
        let line = Line::from(Span::styled(
            status.clone(),
            Style::default()
                .fg(Colors::success())
                .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    // Default: show keybinding hints
    let hints = get_context_hints(app);

    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut parts = vec![
                Span::styled(
                    format!("[{}]", key),
                    Style::default()
                        .fg(Colors::help_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {} ", desc), Style::default().fg(Colors::muted())),
            ];
            if i < hints.len() - 1 {
                parts.push(Span::styled(" | ", Style::default().fg(Colors::muted())));
            }
            parts
        })
        .collect();

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Get context-sensitive keybinding hints.
fn get_context_hints(app: &App) -> Vec<(&'static str, &'static str)> {
    if app.modal.is_some() {
        return vec![("Esc", "Close"), ("j/k", "Navigate")];
    }

    match app.phase {
        AppPhase::Ready => vec![
            ("Enter", "Start"),
            ("c", "Config"),
            ("?", "Help"),
            ("q", "Quit"),
        ],
        AppPhase::DataLoading => vec![
            ("?", "Help"),
            ("q", "Quit"),
        ],
        AppPhase::Running => vec![
            ("Space", "Pause"),
            ("j/k", "Navigate"),
            ("h/l", "Tabs"),
            ("e", "Export"),
            ("?", "Help"),
        ],
        AppPhase::Paused => vec![
            ("Space", "Resume"),
            ("Ctrl+s", "Stop"),
            ("j/k", "Navigate"),
            ("?", "Help"),
        ],
        AppPhase::Completed => vec![
            ("Enter", "Restart"),
            ("j/k", "Navigate"),
            ("e", "Export"),
            ("?", "Help"),
            ("q", "Quit"),
        ],
    }
}
