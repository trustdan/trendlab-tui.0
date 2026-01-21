//! TrendLab TUI
//!
//! Terminal user interface built with Ratatui.
//!
//! # Design Principles
//!
//! - Vim-native keybindings
//! - Semantic color system
//! - State lives centrally; panels render, don't own
//! - YOLO-first: Press Enter to start research
//!
//! # Architecture
//!
//! ```text
//! +-----------+     +--------+     +-------+
//! |  Events   | --> | Action | --> |  App  |
//! +-----------+     +--------+     +-------+
//!                                      |
//!                                      v
//!                                  +-------+
//!                                  |  UI   |
//!                                  +-------+
//! ```
//!
//! Events are converted to Actions via keybindings.
//! Actions are applied to the central App state.
//! The UI renders from App state (panels don't own state).
//!
//! # Modules
//!
//! - [`action`]: Action enum for state mutations
//! - [`app`]: Central application state
//! - [`colors`]: Semantic color system
//! - [`event`]: Async event loop with crossterm
//! - [`keybindings`]: Vim-native key mappings
//! - [`ui`]: Panel rendering

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod action;
pub mod app;
pub mod colors;
pub mod event;
pub mod keybindings;
pub mod runner;
pub mod ui;

// Re-exports for convenience
pub use action::Action;
pub use app::{App, AppPhase, Modal};
pub use colors::Colors;
pub use event::{Event, EventHandler, DEFAULT_TICK_RATE};
pub use keybindings::{key_to_action, KeyContext};
pub use runner::{BacktestRunner, IterationResult};
