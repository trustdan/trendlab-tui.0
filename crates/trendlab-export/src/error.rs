//! Export errors.

use thiserror::Error;

/// Errors that can occur during export operations.
#[derive(Debug, Error)]
pub enum ExportError {
    /// Failed to serialize artifact.
    #[error("Failed to serialize artifact: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Failed to write file.
    #[error("Failed to write file: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to format output.
    #[error("Failed to format output: {0}")]
    Fmt(#[from] std::fmt::Error),

    /// Unsupported component for Pine Script generation.
    #[error("Unsupported component for Pine export: {component}")]
    UnsupportedComponent {
        /// The component that is not supported.
        component: String,
    },

    /// Missing required parameter.
    #[error("Missing required parameter: {param} for component {component}")]
    MissingParam {
        /// Component name.
        component: String,
        /// Parameter name.
        param: String,
    },
}

/// Result type for export operations.
pub type ExportResult<T> = Result<T, ExportError>;
