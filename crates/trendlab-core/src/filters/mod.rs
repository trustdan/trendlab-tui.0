//! Signal Filters for regime-based signal gating.
//!
//! Filters gate entry signals based on market conditions and can force
//! exit of positions when conditions deteriorate.

mod adx;
mod ma_regime;
mod volatility;

pub use adx::AdxFilter;
pub use ma_regime::MaRegimeFilter;
pub use volatility::VolatilityFilter;
