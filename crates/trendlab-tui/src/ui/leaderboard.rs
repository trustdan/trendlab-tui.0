//! Leaderboard panel with tabs.
//!
//! Displays ranked strategies across four leaderboards:
//! - Signal Quality
//! - Position Management
//! - Execution Sensitivity
//! - Overall

use crate::app::App;
use crate::colors::Colors;
use crate::ui::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, Paragraph, Tabs};
use trendlab_yolo::leaderboard::LeaderboardType;

/// Render the leaderboard panel.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel_block("Leaderboard", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create layout for tabs and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Tabs
            Constraint::Min(1),    // List
        ])
        .split(inner);

    // Render tabs
    render_tabs(frame, app, chunks[0]);

    // Render list
    render_list(frame, app, chunks[1]);
}

/// Render the leaderboard tabs.
fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec!["Signal", "PM", "Exec", "Overall"];
    let selected = match app.selected_leaderboard {
        LeaderboardType::SignalQuality => 0,
        LeaderboardType::PositionManagement => 1,
        LeaderboardType::ExecutionSensitivity => 2,
        LeaderboardType::Overall => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .select(selected)
        .style(Style::default().fg(Colors::tab_inactive()))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Colors::tab_active())
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    frame.render_widget(tabs, area);
}

/// Render the leaderboard list.
fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let entries = app.leaderboard_entries();

    if entries.is_empty() {
        let text = Paragraph::new("No results yet")
            .style(Style::default().fg(Colors::muted()))
            .alignment(Alignment::Center);
        frame.render_widget(text, area);
        return;
    }

    // Calculate visible range
    let list_height = area.height as usize;
    let selected = app.selected_entry_index;
    let start = if selected >= list_height {
        selected - list_height + 1
    } else {
        0
    };

    // Create list items
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .skip(start)
        .take(list_height)
        .map(|(idx, entry)| {
            let is_selected = idx == selected;

            // Format entry
            let rank = entry.rank;
            let description = &entry.genome.description();
            let score = entry.robustness.score;

            // Truncate description to fit
            let max_desc_len = (area.width as usize).saturating_sub(15);
            let truncated_desc = if description.len() > max_desc_len {
                format!("{}...", &description[..max_desc_len.saturating_sub(3)])
            } else {
                description.to_string()
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{:>3}. ", rank),
                    Style::default().fg(Colors::muted()),
                ),
                Span::styled(
                    truncated_desc,
                    Style::default().fg(if is_selected {
                        Color::White
                    } else {
                        Colors::muted()
                    }),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:.3}", score),
                    Style::default()
                        .fg(Colors::for_sharpe(score))
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Colors::selection_bg())
                    .fg(Colors::selection_fg())
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, area);
}

/// Get a short name for a leaderboard type.
pub fn short_name(lb_type: LeaderboardType) -> &'static str {
    match lb_type {
        LeaderboardType::SignalQuality => "Signal",
        LeaderboardType::PositionManagement => "PM",
        LeaderboardType::ExecutionSensitivity => "Exec",
        LeaderboardType::Overall => "Overall",
    }
}
