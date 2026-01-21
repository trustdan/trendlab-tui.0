//! UI module - main layout and component rendering.
//!
//! The UI is composed of:
//! - Header: title, tabs, phase, stats
//! - Main content: varies by active tab (YOLO/Results/Chart/Help)
//! - Status bar: keybinding hints
//! - Modals: export, config (overlays)

pub mod chart;
pub mod detail;
pub mod export;
pub mod header;
pub mod help;
pub mod leaderboard;
pub mod progress;
pub mod results;
pub mod status;

use crate::action::Tab;
use crate::app::App;
use crate::colors::Colors;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Render the entire UI.
pub fn render(frame: &mut Frame, app: &App) {
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header (tab bar)
            Constraint::Min(10),    // Main content
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());

    // Header with tab bar
    header::render(frame, app, chunks[0]);

    // Render main content based on active tab
    match app.active_tab {
        Tab::Yolo => render_yolo_tab(frame, app, chunks[1]),
        Tab::Results => results::render(frame, app, chunks[1]),
        Tab::Chart => chart::render(frame, app, chunks[1]),
        Tab::Help => help::render_panel(frame, app, chunks[1]),
    }

    // Status bar
    status::render(frame, app, chunks[2]);

    // Modal overlay (if any)
    if let Some(modal) = app.modal {
        render_modal(frame, app, modal);
    }
}

/// Render the YOLO tab (progress + leaderboard + detail).
fn render_yolo_tab(frame: &mut Frame, app: &App, area: Rect) {
    // Split vertically: main content and detail panel
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // Main content
            Constraint::Length(4),  // Detail panel
        ])
        .split(area);

    // Main content area (split horizontally)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // Progress panel (fixed width)
            Constraint::Min(40),    // Leaderboard panel (flexible)
        ])
        .split(chunks[0]);

    // Progress panel
    progress::render(frame, app, main_chunks[0]);

    // Leaderboard panel
    leaderboard::render(frame, app, main_chunks[1]);

    // Detail panel
    detail::render(frame, app, chunks[1]);
}

/// Render a modal overlay.
fn render_modal(frame: &mut Frame, app: &App, modal: crate::app::Modal) {
    let area = frame.area();

    // Calculate centered modal area (60% width, 70% height)
    let modal_width = (area.width * 60 / 100).clamp(40, 80);
    let modal_height = (area.height * 70 / 100).clamp(10, 30);

    let modal_area = Rect {
        x: (area.width - modal_width) / 2,
        y: (area.height - modal_height) / 2,
        width: modal_width,
        height: modal_height,
    };

    // Clear the modal area
    frame.render_widget(Clear, modal_area);

    // Render the specific modal
    match modal {
        crate::app::Modal::Help => help::render(frame, app, modal_area),
        crate::app::Modal::Export => export::render(frame, app, modal_area),
        crate::app::Modal::Config => render_config_modal(frame, app, modal_area),
    }
}

/// Render the config modal.
fn render_config_modal(frame: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Colors::border_focused()));

    let content = "Configuration options:\n\n\
        Warmup iterations: 50\n\
        Max iterations: 1000\n\
        Max duration: 5 minutes\n\n\
        (Config editing not yet implemented)\n\n\
        Press Esc to close.";

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Create a styled block for panels.
pub fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let border_color = if focused {
        Colors::border_focused()
    } else {
        Colors::border_unfocused()
    };

    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
}
