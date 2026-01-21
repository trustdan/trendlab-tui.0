//! Position manager implementations.
//!
//! Each position manager handles open trades: trailing stops, targets, time exits.
//! They MUST declare their exit reference mode and initialize state at entry.

mod atr_trailing;
mod breakeven_trail;
mod chandelier;
mod donchian_exit;
mod fixed_stop;
mod keltner_exit;
mod max_holding;
mod percent_trailing;
mod sar_exit;
mod time_decay;

pub use atr_trailing::AtrTrailingStop;
pub use breakeven_trail::BreakevenThenTrail;
pub use chandelier::ChandelierExit;
pub use donchian_exit::DonchianExit;
pub use fixed_stop::FixedStop;
pub use keltner_exit::KeltnerExit;
pub use max_holding::MaxHoldingPeriod;
pub use percent_trailing::PercentTrailing;
pub use sar_exit::SarExit;
pub use time_decay::TimeDecayStop;
