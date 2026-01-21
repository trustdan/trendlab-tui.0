//! Detail panel for selected strategy.
//!
//! Shows component breakdown and metrics for the selected leaderboard entry.

use crate::app::App;
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render the detail panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Details", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(entry) = app.selected_entry() {
        render_entry_detail(frame, entry, inner);
    } else {
        render_empty_state(frame, inner);
    }
}

/// Render details for a selected entry.
fn render_entry_detail(frame: &mut Frame, entry: &trendlab_yolo::LeaderboardEntry, area: Rect) {
    let genome = &entry.genome;
    let robustness = &entry.robustness;

    // Build detail lines
    let mut spans = Vec::new();

    // Signal generator
    spans.push(Span::styled("SG: ", Style::default().fg(Colors::help_header())));
    spans.push(Span::styled(
        format!("{}", genome.signal_generator.id),
        Style::default().fg(Color::White),
    ));

    // Parameters (if any)
    if !genome.signal_generator.params.is_empty() {
        let params: Vec<String> = genome
            .signal_generator
            .params
            .iter()
            .map(|(k, v)| format!("{}={:?}", k, v))
            .collect();
        spans.push(Span::styled(
            format!("({})", params.join(", ")),
            Style::default().fg(Colors::muted()),
        ));
    }

    spans.push(Span::raw("  "));

    // Position manager
    spans.push(Span::styled("PM: ", Style::default().fg(Colors::help_header())));
    spans.push(Span::styled(
        format!("{}", genome.position_manager.id),
        Style::default().fg(Color::White),
    ));

    if !genome.position_manager.params.is_empty() {
        let params: Vec<String> = genome
            .position_manager
            .params
            .iter()
            .map(|(k, v)| format!("{}={:?}", k, v))
            .collect();
        spans.push(Span::styled(
            format!("({})", params.join(", ")),
            Style::default().fg(Colors::muted()),
        ));
    }

    spans.push(Span::raw("  "));

    // Execution model
    spans.push(Span::styled("EM: ", Style::default().fg(Colors::help_header())));
    spans.push(Span::styled(
        format!("{}", genome.execution_model.id),
        Style::default().fg(Color::White),
    ));

    spans.push(Span::raw("  "));

    // Metrics
    spans.push(Span::styled("| ", Style::default().fg(Colors::muted())));

    spans.push(Span::styled("Score: ", Style::default().fg(Colors::muted())));
    spans.push(Span::styled(
        format!("{:.3}", robustness.score),
        Style::default()
            .fg(Colors::for_sharpe(robustness.score))
            .add_modifier(Modifier::BOLD),
    ));

    spans.push(Span::raw("  "));

    spans.push(Span::styled("Sharpe: ", Style::default().fg(Colors::muted())));
    spans.push(Span::styled(
        format!("{:.2}", robustness.median_sharpe),
        Style::default().fg(Colors::for_sharpe(robustness.median_sharpe)),
    ));

    spans.push(Span::raw("  "));

    spans.push(Span::styled("WR: ", Style::default().fg(Colors::muted())));
    spans.push(Span::styled(
        format!("{:.0}%", robustness.avg_hit_rate * 100.0),
        Style::default().fg(Colors::for_win_rate(robustness.avg_hit_rate)),
    ));

    spans.push(Span::raw("  "));

    spans.push(Span::styled("DD: ", Style::default().fg(Colors::muted())));
    spans.push(Span::styled(
        format!("{:.1}%", robustness.worst_drawdown * 100.0),
        Style::default().fg(Colors::for_drawdown(robustness.worst_drawdown)),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Render empty state when no entry is selected.
fn render_empty_state(frame: &mut Frame, area: Rect) {
    let text = Paragraph::new("Select an entry to view details")
        .style(Style::default().fg(Colors::muted()))
        .alignment(Alignment::Center);
    frame.render_widget(text, area);
}
