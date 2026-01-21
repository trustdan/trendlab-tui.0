//! TrendLab - Trend-Following Research Lab
//!
//! A terminal-native backtesting lab for exploring trend-following strategies
//! via structural Monte Carlo (YOLO mode).
//!
//! # Usage
//!
//! ```bash
//! cargo run --release
//! ```
//!
//! Press Enter to start YOLO research mode.

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::panic;
use trendlab_tui::{
    action::Action,
    app::App,
    event::{Event, EventHandler, DEFAULT_TICK_RATE},
    keybindings::key_to_action,
    ui,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up panic handler to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before printing panic
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Initialize terminal
    let mut terminal = setup_terminal()?;

    // Create application state
    let mut app = App::new();

    // Create event handler
    let mut events = EventHandler::new(DEFAULT_TICK_RATE);

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &mut events).await;

    // Restore terminal
    restore_terminal()?;

    // Return result
    result
}

/// Set up the terminal for TUI rendering.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Run the main application loop.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &mut EventHandler,
) -> Result<()> {
    loop {
        // Render the UI
        terminal.draw(|frame| {
            // Update visible items based on terminal size
            app.set_visible_items(frame.area().height);
            ui::render(frame, app);
        })?;

        // Handle events
        match events.next().await? {
            Event::Key(key) => {
                let context = app.key_context();
                let action = key_to_action(key, context);
                app.apply_action(action);
            }
            Event::Tick => {
                app.apply_action(Action::Tick);
            }
            Event::Resize(_, height) => {
                app.set_visible_items(height);
            }
            Event::Mouse(_) => {
                // Mouse events not handled yet
            }
            Event::Error(e) => {
                app.error_message = Some(e);
            }
        }

        // Check if we should quit
        if app.should_quit {
            return Ok(());
        }
    }
}
