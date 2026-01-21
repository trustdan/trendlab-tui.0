//! Backtest runner for the TUI.
//!
//! This module bridges the YOLO session with actual backtest execution.
//! It manages test data, runs backtests one at a time on tick, and reports
//! results back to the session.
//!
//! # Architecture
//!
//! The runner is designed to be non-blocking:
//! - Data is pre-loaded once when the session starts
//! - Each tick runs at most one backtest iteration
//! - Rate limiting prevents running on every frame
//!
//! # Usage
//!
//! ```ignore
//! let mut runner = BacktestRunner::new();
//!
//! // On session start
//! runner.initialize();
//!
//! // On each tick (called from App::on_tick)
//! if let Some(result) = runner.run_iteration(&mut session) {
//!     // Result was reported to session
//! }
//! ```

use chrono::NaiveDate;
use rand::Rng;
use trendlab_core::{BacktestConfig, BacktestEngine, Bar, Metrics};
use trendlab_yolo::{Genome, YoloSession};

/// State for backtest execution.
#[derive(Debug)]
pub struct BacktestRunner {
    /// Pre-loaded price bars
    bars: Vec<Bar>,
    /// Pre-computed ATR values (same length as bars)
    atr: Vec<f64>,
    /// Pre-computed ADX values (same length as bars)
    adx: Vec<f64>,
    /// Backtest engine configuration
    engine_config: BacktestConfig,
    /// Tick counter for rate limiting
    tick_counter: u64,
    /// Ticks between backtest runs (rate limiting)
    ticks_per_iteration: u64,
    /// Whether the runner is initialized
    initialized: bool,
}

/// Result of a single backtest iteration.
#[derive(Debug, Clone)]
pub struct IterationResult {
    /// The genome that was tested
    pub genome: Genome,
    /// Performance metrics from the backtest
    pub metrics: Metrics,
    /// Number of trades executed
    pub trade_count: usize,
}

impl Default for BacktestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BacktestRunner {
    /// Create a new backtest runner.
    pub fn new() -> Self {
        Self {
            bars: Vec::new(),
            atr: Vec::new(),
            adx: Vec::new(),
            engine_config: BacktestConfig::default(),
            tick_counter: 0,
            ticks_per_iteration: 3, // ~10 iterations/sec at 30fps
            initialized: false,
        }
    }

    /// Create a runner with custom rate limiting.
    ///
    /// # Arguments
    /// * `ticks_per_iteration` - Number of ticks between backtest runs
    pub fn with_rate_limit(ticks_per_iteration: u64) -> Self {
        Self {
            ticks_per_iteration,
            ..Self::new()
        }
    }

    /// Initialize the runner with mock data.
    ///
    /// This generates synthetic trending data suitable for testing
    /// trend-following strategies.
    pub fn initialize(&mut self) {
        // Generate 500 bars (~2 years of daily data)
        let (bars, atr, adx) = generate_trending_bars(500, 100.0);
        self.bars = bars;
        self.atr = atr;
        self.adx = adx;
        self.initialized = true;
        self.tick_counter = 0;
    }

    /// Initialize with custom data.
    ///
    /// This allows using real market data when available.
    pub fn initialize_with_data(&mut self, bars: Vec<Bar>, atr: Vec<f64>, adx: Vec<f64>) {
        assert_eq!(
            bars.len(),
            atr.len(),
            "ATR length must match bars length"
        );
        assert_eq!(
            bars.len(),
            adx.len(),
            "ADX length must match bars length"
        );

        self.bars = bars;
        self.atr = atr;
        self.adx = adx;
        self.initialized = true;
        self.tick_counter = 0;
    }

    /// Check if the runner is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Reset the runner state.
    pub fn reset(&mut self) {
        self.bars.clear();
        self.atr.clear();
        self.adx.clear();
        self.initialized = false;
        self.tick_counter = 0;
    }

    /// Run one iteration if rate limit allows.
    ///
    /// Returns `Some(result)` if a backtest was run and reported to the session.
    /// Returns `None` if:
    /// - Not initialized
    /// - Rate limited (not enough ticks since last run)
    /// - Session is not running
    /// - No more samples available
    pub fn run_iteration(&mut self, session: &mut YoloSession) -> Option<IterationResult> {
        // Check initialization
        if !self.initialized {
            return None;
        }

        // Check session is running
        if !session.is_running() {
            return None;
        }

        // Rate limiting
        self.tick_counter += 1;
        if self.tick_counter < self.ticks_per_iteration {
            return None;
        }
        self.tick_counter = 0;

        // Get next genome to test
        let genome = session.next_sample()?;

        // Create strategy from genome
        let strategy_result = session.registry().create_strategy(&genome);
        let mut strategy = match strategy_result {
            Ok(s) => s,
            Err(_e) => {
                // Skip invalid genomes - report as invalid result
                let empty_metrics = Metrics::default();
                session.report_result(genome.clone(), empty_metrics.clone());
                return Some(IterationResult {
                    genome,
                    metrics: empty_metrics,
                    trade_count: 0,
                });
            }
        };

        // Run backtest
        let engine = BacktestEngine::new(self.engine_config.clone());
        let result = engine.run(&mut strategy, &self.bars, &self.atr, &self.adx);

        // Report result to session
        session.report_result(genome.clone(), result.metrics.clone());

        Some(IterationResult {
            genome,
            metrics: result.metrics,
            trade_count: result.trades.len(),
        })
    }

    /// Get the number of bars in the test data.
    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }

    /// Get a reference to the bars (for debugging/display).
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }
}

/// Generate synthetic trending bars for testing.
///
/// Creates price data with realistic characteristics:
/// - Trending behavior with regime changes
/// - Realistic ATR and ADX values
/// - No lookahead bias (each bar is independent)
///
/// # Arguments
/// * `count` - Number of bars to generate
/// * `base_price` - Starting price level
///
/// # Returns
/// Tuple of (bars, atr, adx) vectors
pub fn generate_trending_bars(count: usize, base_price: f64) -> (Vec<Bar>, Vec<f64>, Vec<f64>) {
    let mut bars = Vec::with_capacity(count);
    let mut atr = Vec::with_capacity(count);
    let mut adx = Vec::with_capacity(count);
    let mut rng = rand::rng();

    let start_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();

    // State for price generation
    let mut price = base_price;
    let mut trend: f64 = 0.0;
    let mut volatility = 0.015; // 1.5% daily volatility

    for i in 0..count {
        // Regime changes every ~50 bars on average
        if rng.random::<f64>() < 0.02 {
            trend = rng.random_range(-0.003..0.003); // -0.3% to +0.3% daily drift
            volatility = rng.random_range(0.01..0.03); // 1% to 3% volatility
        }

        // Generate OHLC
        let daily_return = trend + rng.random_range(-volatility..volatility);
        let open = price;
        let close = price * (1.0 + daily_return);

        // High/low with realistic intraday range
        let intraday_range = price * volatility * rng.random_range(0.5..1.5);
        let high = open.max(close) + intraday_range * rng.random_range(0.3..0.7);
        let low = open.min(close) - intraday_range * rng.random_range(0.3..0.7);

        // Ensure low doesn't go negative
        let low = low.max(1.0);

        // Create bar
        let date = start_date + chrono::Duration::days(i as i64);
        bars.push(Bar::new(
            date,
            open,
            high,
            low,
            close,
            rng.random_range(500_000..2_000_000),
            i,
        ));

        // ATR: approximate with range * smoothing
        let range = high - low;
        let atr_value = if i == 0 {
            range
        } else {
            // Exponential smoothing
            atr[i - 1] * 0.9 + range * 0.1
        };
        atr.push(atr_value);

        // ADX: trending strength indicator
        // Higher when trend is stronger
        let trend_strength = (trend.abs() / volatility * 100.0).min(50.0);
        let adx_noise = rng.random_range(-5.0..5.0);
        let adx_value = 15.0 + trend_strength + adx_noise;
        adx.push(adx_value.clamp(10.0, 60.0));

        // Update price for next iteration
        price = close;
    }

    (bars, atr, adx)
}

/// Generate a simple uptrend for testing.
///
/// This creates a predictable uptrend useful for verifying strategy behavior.
pub fn generate_simple_uptrend(count: usize, base_price: f64, daily_return: f64) -> (Vec<Bar>, Vec<f64>, Vec<f64>) {
    let mut bars = Vec::with_capacity(count);
    let mut atr = Vec::with_capacity(count);
    let mut adx = Vec::with_capacity(count);

    let start_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
    let mut price = base_price;

    for i in 0..count {
        let open = price;
        let close = price * (1.0 + daily_return);
        let high = close * 1.005; // 0.5% above close
        let low = open * 0.995;   // 0.5% below open

        let date = start_date + chrono::Duration::days(i as i64);
        bars.push(Bar::new(date, open, high, low, close, 1_000_000, i));

        // Constant ATR and ADX for predictability
        atr.push(price * 0.02); // 2% of price
        adx.push(30.0);         // Strong trend

        price = close;
    }

    (bars, atr, adx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trendlab_yolo::{ComponentRegistry, SessionConfig, YoloSession};

    #[test]
    fn test_runner_creation() {
        let runner = BacktestRunner::new();
        assert!(!runner.is_initialized());
        assert_eq!(runner.bar_count(), 0);
    }

    #[test]
    fn test_runner_initialization() {
        let mut runner = BacktestRunner::new();
        runner.initialize();

        assert!(runner.is_initialized());
        assert_eq!(runner.bar_count(), 500);
        assert_eq!(runner.bars().len(), 500);
    }

    #[test]
    fn test_runner_custom_data() {
        let (bars, atr, adx) = generate_simple_uptrend(100, 50.0, 0.001);

        let mut runner = BacktestRunner::new();
        runner.initialize_with_data(bars, atr, adx);

        assert!(runner.is_initialized());
        assert_eq!(runner.bar_count(), 100);
    }

    #[test]
    fn test_runner_reset() {
        let mut runner = BacktestRunner::new();
        runner.initialize();
        assert!(runner.is_initialized());

        runner.reset();
        assert!(!runner.is_initialized());
        assert_eq!(runner.bar_count(), 0);
    }

    #[test]
    fn test_runner_rate_limiting() {
        let mut runner = BacktestRunner::with_rate_limit(3);
        runner.initialize();

        let config = SessionConfig {
            warmup_iterations: 5,
            max_iterations: 100,
            ..Default::default()
        };
        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());
        session.start();

        // First two ticks should be rate limited
        assert!(runner.run_iteration(&mut session).is_none());
        assert!(runner.run_iteration(&mut session).is_none());

        // Third tick should run
        let result = runner.run_iteration(&mut session);
        assert!(result.is_some());
    }

    #[test]
    fn test_runner_session_integration() {
        let mut runner = BacktestRunner::with_rate_limit(1); // No rate limiting
        runner.initialize();

        let config = SessionConfig {
            warmup_iterations: 3,
            max_iterations: 10,
            ..Default::default()
        };
        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());
        session.start();

        // Run a few iterations
        let mut results = Vec::new();
        for _ in 0..5 {
            if let Some(result) = runner.run_iteration(&mut session) {
                results.push(result);
            }
        }

        assert!(!results.is_empty());
        assert!(session.stats().iterations > 0);
    }

    #[test]
    fn test_generate_trending_bars() {
        let (bars, atr, adx) = generate_trending_bars(100, 100.0);

        assert_eq!(bars.len(), 100);
        assert_eq!(atr.len(), 100);
        assert_eq!(adx.len(), 100);

        // Verify bar properties
        for bar in &bars {
            assert!(bar.high >= bar.low);
            assert!(bar.high >= bar.open);
            assert!(bar.high >= bar.close);
            assert!(bar.low <= bar.open);
            assert!(bar.low <= bar.close);
        }

        // Verify ATR is positive
        for a in &atr {
            assert!(*a > 0.0);
        }

        // Verify ADX is in reasonable range
        for a in &adx {
            assert!(*a >= 10.0 && *a <= 60.0);
        }
    }

    #[test]
    fn test_generate_simple_uptrend() {
        let (bars, atr, adx) = generate_simple_uptrend(50, 100.0, 0.01);

        assert_eq!(bars.len(), 50);

        // Verify uptrend: last close should be higher than first open
        let first_open = bars[0].open;
        let last_close = bars[49].close;
        assert!(last_close > first_open);

        // ATR values should all be positive and relatively stable (proportional to price)
        assert!(atr.iter().all(|&a| a > 0.0));

        // ADX should be constant
        assert!(adx.iter().all(|&a| (a - 30.0).abs() < 0.01));
    }

    #[test]
    fn test_runner_not_initialized() {
        let mut runner = BacktestRunner::new();

        let config = SessionConfig::default();
        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());
        session.start();

        // Should return None when not initialized
        assert!(runner.run_iteration(&mut session).is_none());
    }

    #[test]
    fn test_runner_session_not_running() {
        let mut runner = BacktestRunner::with_rate_limit(1);
        runner.initialize();

        let config = SessionConfig::default();
        let session = YoloSession::new(config, ComponentRegistry::with_defaults());
        // Note: session not started

        // Should return None when session not running
        let mut session = session;
        assert!(runner.run_iteration(&mut session).is_none());
    }
}
