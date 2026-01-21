//! TrendLab Export
//!
//! Pine Script generation, strategy artifacts, and reproducibility bundles.
//!
//! # Responsibilities
//!
//! - Generate Pine Script v6 from strategy configs
//! - Create StrategyArtifact JSON with full specification
//! - Include parity test vectors for validation
//! - Export reproducibility bundles (config + seed + manifest)
//!
//! # Parity Testing
//!
//! Every export includes test vectors so users can verify the Pine Script
//! produces identical trades to the Rust backtest.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod artifact;
mod convert;
mod error;
mod pine;

pub use artifact::*;
pub use convert::*;
pub use error::*;
pub use pine::*;

/// Schema version for strategy artifacts.
pub const SCHEMA_VERSION: &str = "1.0.0";
