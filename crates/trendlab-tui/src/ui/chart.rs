//! Chart panel - equity curve visualization.
//!
//! Displays the equity curve for the selected strategy using ratatui's Chart widget.

use crate::app::App;
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};

/// Render the chart panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Chart", true);

    // Get selected entry and its equity curve
    if let Some(entry) = app.selected_entry() {
        if let Some(equity_curve) = app.selected_equity_curve() {
            render_equity_chart(frame, entry, equity_curve, block, area);
        } else {
            // Entry exists but no cached equity curve
            render_no_curve_message(frame, entry, block, area);
        }
    } else {
        render_no_selection_message(frame, block, area);
    }
}

/// Render the equity curve chart.
fn render_equity_chart(
    frame: &mut Frame,
    entry: &trendlab_yolo::LeaderboardEntry,
    equity_curve: &[f64],
    block: Block,
    area: Rect,
) {
    if equity_curve.is_empty() {
        render_no_curve_message(frame, entry, block, area);
        return;
    }

    // Convert equity curve to chart data points
    let data: Vec<(f64, f64)> = equity_curve
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();

    // Calculate Y-axis bounds
    let min_equity = equity_curve.iter().fold(f64::MAX, |a, &b| a.min(b));
    let max_equity = equity_curve.iter().fold(f64::MIN, |a, &b| a.max(b));
    let y_padding = (max_equity - min_equity) * 0.1;
    let y_min = min_equity - y_padding;
    let y_max = max_equity + y_padding;

    // X-axis bounds
    let x_max = (equity_curve.len() - 1) as f64;

    // Create dataset
    let dataset = Dataset::default()
        .name(format!(
            "{} (Score: {:.3})",
            entry.genome.signal_generator.id, entry.robustness.score
        ))
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Colors::accent()))
        .data(&data);

    // Calculate axis labels
    let y_labels = create_y_labels(y_min, y_max);
    let x_labels = create_x_labels(0.0, x_max);

    // Create chart
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(format!(" Equity: {} ", &entry.fingerprint[..8.min(entry.fingerprint.len())]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Colors::border_focused())),
        )
        .x_axis(
            Axis::default()
                .title("Bar")
                .style(Style::default().fg(Colors::muted()))
                .labels(x_labels)
                .bounds([0.0, x_max]),
        )
        .y_axis(
            Axis::default()
                .title("Equity")
                .style(Style::default().fg(Colors::muted()))
                .labels(y_labels)
                .bounds([y_min, y_max]),
        );

    frame.render_widget(chart, area);
}

/// Create Y-axis labels.
fn create_y_labels(min: f64, max: f64) -> Vec<Span<'static>> {
    let mid = (min + max) / 2.0;
    vec![
        Span::styled(format_currency(min), Style::default().fg(Colors::muted())),
        Span::styled(format_currency(mid), Style::default().fg(Colors::muted())),
        Span::styled(format_currency(max), Style::default().fg(Colors::muted())),
    ]
}

/// Create X-axis labels.
fn create_x_labels(min: f64, max: f64) -> Vec<Span<'static>> {
    let mid = (min + max) / 2.0;
    vec![
        Span::styled(format!("{:.0}", min), Style::default().fg(Colors::muted())),
        Span::styled(format!("{:.0}", mid), Style::default().fg(Colors::muted())),
        Span::styled(format!("{:.0}", max), Style::default().fg(Colors::muted())),
    ]
}

/// Format a value as currency (k for thousands).
fn format_currency(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("${:.1}k", value / 1_000.0)
    } else {
        format!("${:.0}", value)
    }
}

/// Render message when entry exists but no equity curve is cached.
fn render_no_curve_message(
    frame: &mut Frame,
    entry: &trendlab_yolo::LeaderboardEntry,
    block: Block,
    area: Rect,
) {
    let content = format!(
        "Strategy: {}\n\n\
        Signal: {}\n\
        Position Manager: {}\n\
        Score: {:.3}\n\n\
        Equity curve not cached.\n\
        The curve is cached when a strategy is evaluated.\n\n\
        This entry may have been loaded from a previous session.",
        entry.fingerprint,
        entry.genome.signal_generator.id,
        entry.genome.position_manager.id,
        entry.robustness.score,
    );

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render message when no strategy is selected.
fn render_no_selection_message(frame: &mut Frame, block: Block, area: Rect) {
    let content = "No strategy selected.\n\n\
        Run a YOLO session (press Enter in the YOLO tab) \n\
        and select a strategy to view its equity curve.\n\n\
        Navigation:\n\
        - 1: Switch to YOLO tab\n\
        - j/k: Navigate leaderboard\n\
        - 3: Return to Chart tab";

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
