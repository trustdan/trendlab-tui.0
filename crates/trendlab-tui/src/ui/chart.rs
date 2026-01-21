//! Chart panel - equity curve visualization.
//!
//! Displays the equity curve for the selected strategy.

use crate::app::App;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render the chart panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Chart", true);

    // Get selected entry for display
    let content = if let Some(entry) = app.selected_entry() {
        format!(
            "Equity Curve for: {}\n\n\
            Signal: {}\n\
            Position Manager: {}\n\
            Score: {:.3}\n\n\
            (Equity curve visualization coming soon)\n\n\
            Use j/k to select a strategy in the YOLO tab.",
            entry.fingerprint,
            entry.genome.signal_generator.id,
            entry.genome.position_manager.id,
            entry.robustness.score,
        )
    } else {
        "No strategy selected.\n\n\
        Run a YOLO session (press Enter in the YOLO tab) \n\
        and select a strategy to view its equity curve.\n\n\
        Navigation:\n\
        - 1: Switch to YOLO tab\n\
        - j/k: Navigate leaderboard\n\
        - 3: Return to Chart tab"
            .to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
