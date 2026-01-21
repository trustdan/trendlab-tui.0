//! YOLO progress panel rendering.
//!
//! Displays session progress, phase, iteration counts, and scores.

use crate::app::{App, AppPhase};
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use trendlab_yolo::SessionPhase;

/// Render the progress panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("YOLO Progress", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.session.is_none() {
        render_ready_state(frame, inner);
        return;
    }

    // Create layout for progress content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Phase
            Constraint::Length(2), // Progress bar
            Constraint::Length(1), // Iterations
            Constraint::Length(1), // Valid/Invalid
            Constraint::Length(1), // Best score
            Constraint::Length(1), // Avg score
            Constraint::Min(0),    // Spacer
        ])
        .split(inner);

    // Phase indicator
    render_phase(frame, app, chunks[0]);

    // Progress bar
    render_progress_bar(frame, app, chunks[1]);

    // Statistics
    render_stats(frame, app, &chunks[2..]);
}

/// Render the ready state (before session starts).
fn render_ready_state(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to start",
            Style::default()
                .fg(Colors::info())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "YOLO research mode",
            Style::default().fg(Colors::muted()),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Render the phase indicator.
fn render_phase(frame: &mut Frame, app: &App, area: Rect) {
    let (phase_text, color) = match app.session_phase() {
        SessionPhase::NotStarted => ("Not Started", Colors::muted()),
        SessionPhase::Warmup => ("Warmup", Colors::phase_warmup()),
        SessionPhase::Exploitation => ("Exploitation", Colors::phase_exploitation()),
        SessionPhase::Completed => ("Completed", Colors::phase_completed()),
    };

    let paused = if app.phase == AppPhase::Paused {
        " (Paused)"
    } else {
        ""
    };

    let line = Line::from(vec![
        Span::raw("Phase: "),
        Span::styled(
            format!("{}{}", phase_text, paused),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Render the progress bar.
fn render_progress_bar(frame: &mut Frame, app: &App, area: Rect) {
    let progress = app.session_progress();

    // Create ASCII progress bar
    let bar_width = (area.width as usize).saturating_sub(2);
    let filled = (progress * bar_width as f64) as usize;
    let empty = bar_width.saturating_sub(filled);

    let bar = format!(
        "[{}{}] {:>3}%",
        "#".repeat(filled),
        "-".repeat(empty),
        (progress * 100.0) as u32
    );

    let color = match app.session_phase() {
        SessionPhase::Warmup => Colors::phase_warmup(),
        SessionPhase::Exploitation => Colors::phase_exploitation(),
        SessionPhase::Completed => Colors::phase_completed(),
        _ => Colors::muted(),
    };

    let line = Line::from(Span::styled(bar, Style::default().fg(color)));
    frame.render_widget(Paragraph::new(line), area);
}

/// Render statistics.
fn render_stats(frame: &mut Frame, app: &App, chunks: &[Rect]) {
    if let Some(stats) = app.session_stats() {
        // Iterations
        if !chunks.is_empty() {
            let config = app
                .session
                .as_ref()
                .map(|s| s.config().max_iterations)
                .unwrap_or(0);
            let iter_text = if config > 0 {
                format!("Iterations: {}/{}", stats.iterations, config)
            } else {
                format!("Iterations: {}", stats.iterations)
            };
            frame.render_widget(
                Paragraph::new(iter_text).style(Style::default().fg(Colors::muted())),
                chunks[0],
            );
        }

        // Valid/Invalid
        if chunks.len() > 1 {
            let line = Line::from(vec![
                Span::raw("Valid: "),
                Span::styled(
                    stats.valid_results.to_string(),
                    Style::default().fg(Colors::success()),
                ),
                Span::raw(" Invalid: "),
                Span::styled(
                    stats.invalid_results.to_string(),
                    Style::default().fg(if stats.invalid_results > 0 {
                        Colors::warning()
                    } else {
                        Colors::muted()
                    }),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), chunks[1]);
        }

        // Best score
        if chunks.len() > 2 {
            let line = Line::from(vec![
                Span::raw("Best: "),
                Span::styled(
                    format!("{:.3}", stats.best_score),
                    Style::default()
                        .fg(Colors::for_sharpe(stats.best_score))
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), chunks[2]);
        }

        // Avg score
        if chunks.len() > 3 {
            let line = Line::from(vec![
                Span::raw("Avg: "),
                Span::styled(
                    format!("{:.3}", stats.avg_score),
                    Style::default().fg(Colors::for_sharpe(stats.avg_score)),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), chunks[3]);
        }
    }
}
