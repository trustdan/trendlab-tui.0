//! Async event loop using crossterm.
//!
//! Handles keyboard input, mouse events, and tick events for UI updates.

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Events that the event handler can produce.
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick (for animations and periodic updates)
    Tick,
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize event
    Resize(u16, u16),
    /// Error event
    Error(String),
}

/// Event handler that runs in a background task.
pub struct EventHandler {
    /// Channel receiver for events
    rx: mpsc::UnboundedReceiver<Event>,
    /// Handle to the background task
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate.
    ///
    /// # Arguments
    /// * `tick_rate` - Duration between tick events (e.g., 33ms for ~30fps)
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                // Poll for crossterm events
                let event = if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        // Only handle key press events (ignore release/repeat on Windows)
                        Ok(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                            Some(Event::Key(key))
                        }
                        Ok(CrosstermEvent::Key(_)) => None, // Ignore release/repeat
                        Ok(CrosstermEvent::Mouse(mouse)) => Some(Event::Mouse(mouse)),
                        Ok(CrosstermEvent::Resize(w, h)) => Some(Event::Resize(w, h)),
                        Ok(_) => None, // Ignore other events
                        Err(e) => Some(Event::Error(e.to_string())),
                    }
                } else {
                    // No event, send tick
                    Some(Event::Tick)
                };

                if let Some(evt) = event
                    && tx.send(evt).is_err()
                {
                    // Channel closed, exit loop
                    break;
                }
            }
        });

        Self { rx, _task: task }
    }

    /// Get the next event, blocking until one is available.
    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}

/// Default tick rate (approximately 30 FPS)
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(33);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tick_rate() {
        assert_eq!(DEFAULT_TICK_RATE, Duration::from_millis(33));
    }
}
