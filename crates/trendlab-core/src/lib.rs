//! TrendLab Core
//!
//! Engine spine, traits, types, and metrics for the TrendLab backtester.
//!
//! # Architecture
//!
//! Every strategy is a composition of four independent layers:
//! - [`SignalGenerator`](traits::SignalGenerator) - Entry signal logic
//! - [`PositionManager`](traits::PositionManager) - Trade management and exits
//! - [`ExecutionModel`](traits::ExecutionModel) - Fill simulation
//! - [`SignalFilter`](traits::SignalFilter) - Regime gating
//!
//! Components communicate through the engine, never directly.
//!
//! # The Stickiness Problem
//!
//! In v1, strategies using rolling references (e.g., 52-week high) for BOTH
//! entry AND exit created "sticky" positions. TrendLab v2 prevents this by:
//!
//! 1. Separating signal generation from position management
//! 2. Requiring explicit [`ExitReferenceMode`] declarations
//! 3. Tracking extremes from entry, not globally
//!
//! # Example
//!
//! ```ignore
//! use trendlab_core::prelude::*;
//!
//! // Compose a strategy from independent components
//! let strategy = Strategy {
//!     signal_generator: Box::new(DonchianBreakout::new(20)),
//!     position_manager: Box::new(AtrTrailingStop::new(2.0)),
//!     execution_model: Box::new(NextOpenFill::new(5.0, 1.0)),
//!     signal_filter: None,
//! };
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod engine;
pub mod error;
pub mod execution;
pub mod exit_reference;
pub mod filters;
pub mod market_state;
pub mod param;
pub mod position_managers;
pub mod signals;
pub mod traits;
pub mod types;

/// Re-exports of commonly used items.
pub mod prelude {
    pub use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult, Strategy};
    pub use crate::error::CoreError;
    pub use crate::execution::NextOpenFill;
    pub use crate::exit_reference::ExitReferenceMode;
    pub use crate::filters::{AdxFilter, MaRegimeFilter, VolatilityFilter};
    pub use crate::market_state::MarketState;
    pub use crate::param::{ParamDef, ParamSet, ParamType, ParamValue};
    pub use crate::position_managers::{
        AtrTrailingStop, BreakevenThenTrail, ChandelierExit, DonchianExit, FixedStop,
        KeltnerExit, MaxHoldingPeriod, PercentTrailing, SarExit, TimeDecayStop,
    };
    pub use crate::signals::{
        AroonCrossover, BollingerBreakout, DonchianBreakout, FiftyTwoWeekBreakout, KeltnerBreakout,
        MaCrossover, Momentum, ParabolicSar, RocMomentum, Supertrend, TrendFlip,
    };
    pub use crate::traits::{
        ExecutionModel, NoFilter, PositionManager, SignalFilter, SignalGenerator,
    };
    pub use crate::types::{
        Action, Bar, Direction, ExitReason, FillResult, GapPolicy, Metrics, Order, OrderType,
        Position, Signal, Trade,
    };
}

// Re-export at crate root for convenience
pub use engine::{BacktestConfig, BacktestEngine, BacktestResult, Strategy};
pub use error::CoreError;
pub use execution::NextOpenFill;
pub use exit_reference::ExitReferenceMode;
pub use filters::{AdxFilter, MaRegimeFilter, VolatilityFilter};
pub use market_state::MarketState;
pub use param::{ParamDef, ParamSet, ParamType, ParamValue};
pub use position_managers::{
    AtrTrailingStop, BreakevenThenTrail, ChandelierExit, DonchianExit, FixedStop, KeltnerExit,
    MaxHoldingPeriod, PercentTrailing, SarExit, TimeDecayStop,
};
pub use signals::{
    AroonCrossover, BollingerBreakout, DonchianBreakout, FiftyTwoWeekBreakout, KeltnerBreakout,
    MaCrossover, Momentum, ParabolicSar, RocMomentum, Supertrend, TrendFlip,
};
pub use traits::{ExecutionModel, NoFilter, PositionManager, SignalFilter, SignalGenerator};
pub use types::{
    Action, Bar, Direction, ExitReason, FillResult, GapPolicy, Metrics, Order, OrderType,
    Position, Signal, Trade,
};
