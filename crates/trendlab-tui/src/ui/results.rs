//! Results panel - sortable table of all backtest results.
//!
//! Displays a table view of all strategy results with:
//! - Columns: #, Strategy, Config, Sharpe, CAGR, MaxDD, Hit Rate, Iter
//! - j/k navigation, s to cycle sort column
//! - Enter to view in Chart panel
//! - P to export Pine Script

use crate::app::App;
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

/// Render the results panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Results", true);

    // Get all entries from the current leaderboard
    let entries = app.leaderboard_entries();

    if entries.is_empty() {
        let content = "No results yet.\n\n\
            Run a YOLO session (press Enter in the YOLO tab) \n\
            to see strategy results here.\n\n\
            Navigation:\n\
            - 1: Switch to YOLO tab\n\
            - Enter: Start YOLO session";

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::White))
            .wrap(ratatui::widgets::Wrap { trim: true });

        frame.render_widget(paragraph, area);
        return;
    }

    // Build table rows
    let header = Row::new(vec![
        Cell::from("#").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Signal").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("PM").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Score").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Sharpe").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("MaxDD").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Hit%").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Colors::accent()));

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == app.selected_entry_index;
            let style = if is_selected {
                Style::default()
                    .fg(Colors::selection_fg())
                    .bg(Colors::selection_bg())
            } else {
                Style::default().fg(Color::White)
            };

            Row::new(vec![
                Cell::from(format!("{:>3}", i + 1)),
                Cell::from(format!("{}", entry.genome.signal_generator.id)),
                Cell::from(format!("{}", entry.genome.position_manager.id)),
                Cell::from(format!("{:.3}", entry.robustness.score)),
                Cell::from(format!("{:.2}", entry.robustness.median_sharpe)),
                Cell::from(format!("{:.1}%", entry.robustness.worst_drawdown * 100.0)),
                Cell::from(format!("{:.1}%", entry.robustness.avg_hit_rate * 100.0)),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Min(15),
        Constraint::Min(15),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(Colors::selection_bg()));

    frame.render_widget(table, area);
}
