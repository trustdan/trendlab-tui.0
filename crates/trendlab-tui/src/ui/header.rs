//! Header panel rendering.
//!
//! Displays: title, version, tabs, phase, stats.

use crate::action::Tab;
use crate::app::{App, AppPhase};
use crate::colors::Colors;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use trendlab_yolo::SessionPhase;

/// Render the header panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Colors::border_unfocused()));

    // Build header content
    let title = Span::styled(
        "TrendLab",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    // Tab bar
    let tab_spans = format_tabs(app);

    let phase_span = format_phase(app);
    let stats_span = format_stats(app);

    // Create line with spacing
    let mut spans = vec![
        title,
        Span::raw("  "),
    ];
    spans.extend(tab_spans);
    spans.push(Span::raw("  │  "));
    spans.push(phase_span);
    spans.push(Span::raw("  "));
    spans.push(stats_span);

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Format the tab bar.
fn format_tabs(app: &App) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    for (i, tab) in Tab::all().iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }

        let is_active = *tab == app.active_tab;
        let (style, prefix, suffix) = if is_active {
            (
                Style::default()
                    .fg(Colors::accent())
                    .add_modifier(Modifier::BOLD),
                "[",
                "]",
            )
        } else {
            (Style::default().fg(Colors::muted()), " ", " ")
        };

        // Format: [1] YOLO or  2  Results
        let text = format!("{}{}{} {}", prefix, tab.number(), suffix, tab.name());
        spans.push(Span::styled(text, style));
    }

    spans
}

/// Format the phase indicator with appropriate color.
fn format_phase(app: &App) -> Span<'static> {
    let (text, color) = match app.phase {
        AppPhase::Ready => ("Ready", Colors::muted()),
        AppPhase::DataLoading => ("Loading Data", Colors::info()),
        AppPhase::Running => match app.session_phase() {
            SessionPhase::Warmup => ("Warmup", Colors::phase_warmup()),
            SessionPhase::Exploitation => ("Exploitation", Colors::phase_exploitation()),
            _ => ("Running", Colors::info()),
        },
        AppPhase::Paused => ("Paused", Colors::warning()),
        AppPhase::Completed => ("Completed", Colors::phase_completed()),
    };

    Span::styled(
        format!("[{}]", text),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// Format session stats.
fn format_stats(app: &App) -> Span<'static> {
    if let Some(stats) = app.session_stats() {
        let text = format!(
            "Iter: {} | Valid: {} | Best: {:.2}",
            stats.iterations, stats.valid_results, stats.best_score
        );
        Span::styled(text, Style::default().fg(Colors::muted()))
    } else {
        Span::styled(
            "Press Enter to start YOLO research",
            Style::default().fg(Colors::info()),
        )
    }
}
