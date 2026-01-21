//! Core types for TrendLab.

mod action;
mod bar;
mod direction;
mod fill;
mod metrics;
mod order;
mod position;
mod signal;
mod trade;

pub use action::{Action, ExitReason};
pub use bar::Bar;
pub use direction::Direction;
pub use fill::{FillResult, GapPolicy};
pub use metrics::Metrics;
pub use order::{Order, OrderType};
pub use position::Position;
pub use signal::Signal;
pub use trade::Trade;
