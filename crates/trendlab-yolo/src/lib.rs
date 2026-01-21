//! TrendLab YOLO
//!
//! Structural Monte Carlo search, ranking, and attribution.
//!
//! # YOLO Mode
//!
//! "You Only Look Once" - the discovery engine that explores the combinatorial
//! strategy space (1,760+ structural combinations) to find robust winners.
//!
//! # Architecture
//!
//! The YOLO system samples strategies by varying **structure first** (which
//! components to combine), then parameters. This allows exploration of the full
//! combinatorial space rather than getting stuck in local optima.
//!
//! ## Key Types
//!
//! - [`Genome`]: Encodes a strategy composition (component types + parameters)
//! - [`ComponentRegistry`]: Factory for creating components from IDs
//! - [`RobustnessScorer`]: Computes robustness scores with penalties
//! - [`Leaderboard`]: Ranked list of strategies by robustness
//! - [`StructuralSampler`]: Monte Carlo sampler (structure-first)
//! - [`YoloSession`]: Session state machine (warmup → exploitation)
//!
//! # Invariants
//!
//! - **Invariant E (Multiple Leaderboards)**: Three disentangled leaderboards
//!   (Signal Quality, Position Management, Execution Sensitivity) before
//!   trusting overall winners.
//! - **Determinism**: Same seed + config = identical results.
//! - **Attribution**: Know which component drove performance.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod attribution;
pub mod error;
pub mod genome;
pub mod leaderboard;
pub mod registry;
pub mod robustness;
pub mod sampler;
pub mod session;

pub use attribution::ComponentAttribution;
pub use error::YoloError;
pub use genome::{ComponentConfig, ComponentId, Genome};
pub use leaderboard::{Leaderboard, LeaderboardEntry, LeaderboardSet};
pub use registry::ComponentRegistry;
pub use robustness::{RobustnessConfig, RobustnessScore, RobustnessScorer};
pub use sampler::{SamplerConfig, StructuralSampler};
pub use session::{SessionConfig, SessionPhase, YoloSession};
