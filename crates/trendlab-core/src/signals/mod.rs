//! Signal generator implementations.
//!
//! Each signal generator identifies entry opportunities based on price action
//! and indicator conditions. They produce entry intents, NOT exit logic.
//!
//! # Available Signal Generators
//!
//! 1. [`DonchianBreakout`] - Donchian channel breakout (classic Turtle)
//! 2. [`FiftyTwoWeekBreakout`] - 52-week high/low breakout
//! 3. [`MaCrossover`] - Moving average crossover (Golden/Death cross)
//! 4. [`Supertrend`] - ATR-based trend following
//! 5. [`Momentum`] - Time-series momentum (TSMOM)
//! 6. [`BollingerBreakout`] - Bollinger Bands breakout
//! 7. [`KeltnerBreakout`] - Keltner Channel breakout
//! 8. [`ParabolicSar`] - Parabolic Stop and Reverse
//! 9. [`RocMomentum`] - Rate of Change momentum
//! 10. [`AroonCrossover`] - Aroon indicator crossover
//! 11. [`TrendFlip`] - Trend reversal detection

mod aroon;
mod bollinger;
mod donchian;
mod fifty_two_week;
mod keltner;
mod ma_crossover;
mod momentum;
mod parabolic_sar;
mod roc;
mod supertrend;
mod trend_flip;

pub use aroon::AroonCrossover;
pub use bollinger::BollingerBreakout;
pub use donchian::DonchianBreakout;
pub use fifty_two_week::FiftyTwoWeekBreakout;
pub use keltner::KeltnerBreakout;
pub use ma_crossover::MaCrossover;
pub use momentum::Momentum;
pub use parabolic_sar::ParabolicSar;
pub use roc::RocMomentum;
pub use supertrend::Supertrend;
pub use trend_flip::TrendFlip;
