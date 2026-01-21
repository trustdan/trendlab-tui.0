//! Error types for trendlab-core.

use thiserror::Error;

/// Core errors for the TrendLab engine.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Invalid bar data (e.g., high < low)
    #[error("Invalid bar data: {0}")]
    InvalidBar(String),

    /// Insufficient warmup period.
    #[error("Insufficient warmup: need {required} bars, have {available}")]
    InsufficientWarmup {
        /// Number of bars required for warmup.
        required: usize,
        /// Number of bars available.
        available: usize,
    },

    /// Invalid parameter value.
    #[error("Invalid parameter '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name.
        name: String,
        /// Reason the value is invalid.
        reason: String,
    },

    /// Component state error
    #[error("Component state error: {0}")]
    ComponentState(String),

    /// Execution error
    #[error("Execution error: {0}")]
    Execution(String),
}
