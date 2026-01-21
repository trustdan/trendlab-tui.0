//! Export modal rendering.
//!
//! Displays strategy details and export options for Pine Script generation.

use crate::app::App;
use crate::colors::Colors;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use trendlab_yolo::LeaderboardEntry;

/// Render the export modal.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Export Strategy ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::border_focused()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(entry) = app.selected_entry() {
        render_export_content(frame, entry, inner);
    } else {
        render_no_selection(frame, inner);
    }
}

/// Render export content for a selected entry.
fn render_export_content(frame: &mut Frame, entry: &LeaderboardEntry, area: Rect) {
    let genome = &entry.genome;
    let robustness = &entry.robustness;

    let mut lines = Vec::new();

    // Strategy title
    lines.push(Line::from(Span::styled(
        genome.description(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Signal Generator section
    lines.push(Line::from(Span::styled(
        "Signal Generator",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}", genome.signal_generator.id),
            Style::default().fg(Color::White),
        ),
    ]));

    // Signal Generator parameters
    for (key, value) in &genome.signal_generator.params {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{}: ", key), Style::default().fg(Colors::muted())),
            Span::styled(
                format!("{:?}", value),
                Style::default().fg(Colors::help_key()),
            ),
        ]));
    }
    if genome.signal_generator.params.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("(default parameters)", Style::default().fg(Colors::muted())),
        ]));
    }
    lines.push(Line::from(""));

    // Position Manager section
    lines.push(Line::from(Span::styled(
        "Position Manager",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}", genome.position_manager.id),
            Style::default().fg(Color::White),
        ),
    ]));

    for (key, value) in &genome.position_manager.params {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{}: ", key), Style::default().fg(Colors::muted())),
            Span::styled(
                format!("{:?}", value),
                Style::default().fg(Colors::help_key()),
            ),
        ]));
    }
    if genome.position_manager.params.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("(default parameters)", Style::default().fg(Colors::muted())),
        ]));
    }
    lines.push(Line::from(""));

    // Execution Model section
    lines.push(Line::from(Span::styled(
        "Execution Model",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}", genome.execution_model.id),
            Style::default().fg(Color::White),
        ),
    ]));

    for (key, value) in &genome.execution_model.params {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{}: ", key), Style::default().fg(Colors::muted())),
            Span::styled(
                format!("{:?}", value),
                Style::default().fg(Colors::help_key()),
            ),
        ]));
    }
    if genome.execution_model.params.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("(default parameters)", Style::default().fg(Colors::muted())),
        ]));
    }
    lines.push(Line::from(""));

    // Metrics section
    lines.push(Line::from(Span::styled(
        "Metrics",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));

    // Two-column metrics display
    let sharpe_color = Colors::for_sharpe(robustness.median_sharpe);
    let win_rate_color = Colors::for_win_rate(robustness.avg_hit_rate);
    let dd_color = Colors::for_drawdown(robustness.worst_drawdown);
    let score_color = Colors::for_sharpe(robustness.score);

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Sharpe:  ", Style::default().fg(Colors::muted())),
        Span::styled(
            format!("{:>6.2}", robustness.median_sharpe),
            Style::default().fg(sharpe_color),
        ),
        Span::styled("  |  ", Style::default().fg(Colors::muted())),
        Span::styled("Win Rate: ", Style::default().fg(Colors::muted())),
        Span::styled(
            format!("{:>5.0}%", robustness.avg_hit_rate * 100.0),
            Style::default().fg(win_rate_color),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Runs:    ", Style::default().fg(Colors::muted())),
        Span::styled(
            format!("{:>6}", robustness.num_runs),
            Style::default().fg(Color::White),
        ),
        Span::styled("  |  ", Style::default().fg(Colors::muted())),
        Span::styled("Max DD:   ", Style::default().fg(Colors::muted())),
        Span::styled(
            format!("{:>5.1}%", robustness.worst_drawdown * 100.0),
            Style::default().fg(dd_color),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Robust:  ", Style::default().fg(Colors::muted())),
        Span::styled(
            format!("{:>6.3}", robustness.score),
            Style::default().fg(score_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  |  ", Style::default().fg(Colors::muted())),
        Span::styled("Valid:    ", Style::default().fg(Colors::muted())),
        Span::styled(
            if robustness.is_valid { "  Yes" } else { "   No" },
            Style::default().fg(if robustness.is_valid {
                Colors::success()
            } else {
                Colors::danger()
            }),
        ),
    ]));

    lines.push(Line::from(""));

    // Export Options section
    lines.push(Line::from(Span::styled(
        "Export Options",
        Style::default()
            .fg(Colors::help_header())
            .add_modifier(Modifier::UNDERLINED),
    )));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("[1]", Style::default().fg(Colors::help_key())),
        Span::raw(" Pine Script v6 "),
        Span::styled("(Coming soon)", Style::default().fg(Colors::muted())),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("[2]", Style::default().fg(Colors::help_key())),
        Span::raw(" JSON Artifact"),
    ]));

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("[3]", Style::default().fg(Colors::help_key())),
        Span::raw(" Full Bundle "),
        Span::styled("(Coming soon)", Style::default().fg(Colors::muted())),
    ]));

    lines.push(Line::from(""));

    // Footer
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Colors::muted())),
        Span::styled("Enter", Style::default().fg(Colors::help_key())),
        Span::styled(" to export JSON, ", Style::default().fg(Colors::muted())),
        Span::styled("Esc", Style::default().fg(Colors::help_key())),
        Span::styled(" to cancel", Style::default().fg(Colors::muted())),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Render empty state when no entry is selected.
fn render_no_selection(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No strategy selected",
            Style::default()
                .fg(Colors::warning())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Select a strategy from the leaderboard first.",
            Style::default().fg(Colors::muted()),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Colors::muted())),
            Span::styled("Esc", Style::default().fg(Colors::help_key())),
            Span::styled(" to close", Style::default().fg(Colors::muted())),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_no_crash() {
        // Ensure render function doesn't panic with default state
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(10, 5, 60, 20);
                render(frame, &app, area);
            })
            .unwrap();
    }
}
