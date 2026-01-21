//! Component traits for the TrendLab strategy architecture.
//!
//! Every strategy is composed of four independent layers:
//! - [`SignalGenerator`]: Entry signal logic
//! - [`PositionManager`]: Trade management and exits
//! - [`ExecutionModel`]: Fill simulation
//! - [`SignalFilter`]: Regime gating

mod execution_model;
mod position_manager;
mod signal_filter;
mod signal_generator;

pub use execution_model::ExecutionModel;
pub use position_manager::PositionManager;
pub use signal_filter::{NoFilter, SignalFilter};
pub use signal_generator::SignalGenerator;
