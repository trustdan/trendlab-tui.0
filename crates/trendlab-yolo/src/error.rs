//! YOLO-specific error types.

use thiserror::Error;

/// Errors that can occur during YOLO operations.
#[derive(Debug, Error)]
pub enum YoloError {
    /// Component not found in registry.
    #[error("unknown component: {0}")]
    UnknownComponent(String),

    /// Invalid parameter value.
    #[error("invalid parameter '{param}' for component '{component}': {reason}")]
    InvalidParameter {
        /// Component name
        component: String,
        /// Parameter name
        param: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Session is in wrong phase for operation.
    #[error("invalid session phase: expected {expected}, got {actual}")]
    InvalidPhase {
        /// Expected phase
        expected: String,
        /// Actual phase
        actual: String,
    },

    /// No data available for backtest.
    #[error("no market data available for symbol: {0}")]
    NoData(String),

    /// Core engine error.
    #[error("backtest error: {0}")]
    BacktestError(#[from] trendlab_core::CoreError),

    /// Data fetching error.
    #[error("data error: {0}")]
    DataError(#[from] trendlab_data::DataError),
}
