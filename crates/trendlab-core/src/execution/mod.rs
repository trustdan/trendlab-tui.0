//! Execution model implementations.
//!
//! Each execution model simulates how orders become fills with realistic
//! assumptions about timing, slippage, gaps, and fees.

mod next_open;

pub use next_open::NextOpenFill;
