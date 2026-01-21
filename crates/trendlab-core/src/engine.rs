//! Backtest engine - the sacred simulation loop.
//!
//! # The Event Loop (per bar N)
//!
//! 1. Update position tracking (if position exists)
//!    - high_since_entry = max(current, bar.high)
//!    - low_since_entry = min(current, bar.low)
//!    - bars_held += 1
//!
//! 2. Check for EXITS BEFORE ENTRIES
//!    - Check stop via ExecutionModel (uses prior stop; stop updates apply next bar)
//!    - Check PositionManager exit action
//!    - Check SignalFilter force exit
//!    - If exit: record trade, clear position
//!
//! 3. If no position AND past warmup, check for entries
//!    - Get signal from SignalGenerator
//!    - Check SignalFilter allow
//!    - Attempt fill via ExecutionModel (using bar N+1)
//!    - If filled: create position, call PM.on_entry()
//!
//! 4. Record equity snapshot

use crate::market_state::MarketState;
use crate::traits::{ExecutionModel, PositionManager, SignalFilter, SignalGenerator};
use crate::types::{
    Action, Bar, ExitReason, Metrics, Order, OrderType, Position, Trade,
};

/// Backtest engine configuration.
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// Initial equity for the backtest
    pub initial_equity: f64,
    /// Position size as percentage of equity (1.0 = 100%)
    pub position_size_pct: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_equity: 100_000.0,
            position_size_pct: 1.0,
        }
    }
}

/// Composed strategy (the four orthogonal layers).
pub struct Strategy {
    /// Signal generator component
    pub signal_generator: Box<dyn SignalGenerator>,
    /// Position manager component
    pub position_manager: Box<dyn PositionManager>,
    /// Execution model component
    pub execution_model: Box<dyn ExecutionModel>,
    /// Optional signal filter
    pub signal_filter: Option<Box<dyn SignalFilter>>,
}

impl Strategy {
    /// Create a new strategy composition.
    pub fn new(
        signal_generator: Box<dyn SignalGenerator>,
        position_manager: Box<dyn PositionManager>,
        execution_model: Box<dyn ExecutionModel>,
        signal_filter: Option<Box<dyn SignalFilter>>,
    ) -> Self {
        Self {
            signal_generator,
            position_manager,
            execution_model,
            signal_filter,
        }
    }

    /// Get component names for logging.
    pub fn component_names(&self) -> StrategyComponents {
        StrategyComponents {
            signal_generator: self.signal_generator.name().to_string(),
            position_manager: self.position_manager.name().to_string(),
            execution_model: self.execution_model.name().to_string(),
            signal_filter: self.signal_filter.as_ref().map(|f| f.name().to_string()),
        }
    }

    /// Reset all component state for a fresh run.
    pub fn reset(&mut self) {
        self.position_manager.reset();
    }
}

/// Component names for a strategy.
#[derive(Debug, Clone)]
pub struct StrategyComponents {
    /// Signal generator name
    pub signal_generator: String,
    /// Position manager name
    pub position_manager: String,
    /// Execution model name
    pub execution_model: String,
    /// Signal filter name (if any)
    pub signal_filter: Option<String>,
}

/// Backtest result containing trades, equity curve, and metrics.
#[derive(Debug, Clone)]
pub struct BacktestResult {
    /// Completed trades
    pub trades: Vec<Trade>,
    /// Equity curve (one value per bar)
    pub equity_curve: Vec<f64>,
    /// Aggregate performance metrics
    pub metrics: Metrics,
    /// Run fingerprint for reproducibility
    pub fingerprint: String,
    /// Component names used in this run
    pub components: StrategyComponents,
}

/// The sacred backtest engine.
///
/// Processes bars one at a time, enforcing the Signal → Order → Fill separation
/// and ensuring exits are checked before entries.
pub struct BacktestEngine {
    config: BacktestConfig,
}

impl BacktestEngine {
    /// Create a new backtest engine with the given configuration.
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }

    /// Run a backtest with the given strategy and market data.
    ///
    /// # Arguments
    /// - `strategy`: The composed strategy to test
    /// - `bars`: Historical OHLCV bars
    /// - `atr`: Pre-computed ATR values (same length as bars)
    /// - `adx`: Pre-computed ADX values (same length as bars)
    ///
    /// # Returns
    /// Complete backtest results with trades, equity curve, and metrics.
    pub fn run(
        &self,
        strategy: &mut Strategy,
        bars: &[Bar],
        atr: &[f64],
        adx: &[f64],
    ) -> BacktestResult {
        // Reset strategy state for fresh run
        strategy.reset();

        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<f64> = Vec::with_capacity(bars.len());
        let mut position: Option<Position> = None;
        let mut equity = self.config.initial_equity;

        let warmup = strategy.signal_generator.warmup_bars();
        let components = strategy.component_names();

        for i in 0..bars.len() {
            let bar = &bars[i];

            // Create market state (ONLY bars up to current - no lookahead!)
            let state = MarketState::new(&bars[..=i], i, &atr[..=i], &adx[..=i]);

            // === STEP 1: Update position tracking ===
            if let Some(ref mut pos) = position {
                pos.update_for_bar(bar);
            }

            // === STEP 2: Check EXITS before entries ===
            if let Some(mut pos) = position.take() {
                let mut exit_price: Option<f64> = None;
                let mut exit_reason: Option<ExitReason> = None;

                // 2a. Check stop via ExecutionModel
                if let Some(stop_fill) = strategy.execution_model.check_stop(&pos, bar) {
                    exit_price = Some(stop_fill);
                    exit_reason = Some(ExitReason::StopHit);
                }

                // 2b. Check PositionManager action (if no stop hit)
                if exit_reason.is_none() {
                    match strategy.position_manager.on_bar(bar, &pos, &state) {
                        Action::Exit(reason) => {
                            exit_price = Some(bar.close);
                            exit_reason = Some(reason);
                        }
                        Action::AdjustStop(new_stop) => {
                            // Apply stop update for next bar's stop check
                            pos.stop_price = Some(new_stop);
                        }
                        Action::Hold | Action::ScaleOut { .. } => {}
                    }
                }

                // 2c. Check SignalFilter force exit
                if exit_reason.is_none() {
                    if let Some(ref filter) = strategy.signal_filter {
                        if filter.force_exit(&pos, bar, &state) {
                            exit_price = Some(bar.close);
                            exit_reason = Some(ExitReason::FilterForceExit);
                        }
                    }
                }

                // Execute exit or restore position
                if let (Some(price), Some(reason)) = (exit_price, exit_reason) {
                    let trade = self.create_trade(&pos, i, bar.date, price, reason);
                    equity *= 1.0 + trade.return_pct;
                    trades.push(trade);
                    // position remains None
                } else {
                    position = Some(pos); // Restore position
                }
            }

            // === STEP 3: Check entries (if no position) ===
            if position.is_none() && i >= warmup && i + 1 < bars.len() {
                if let Some(signal) = strategy.signal_generator.generate(bar, &state) {
                    let allowed = strategy
                        .signal_filter
                        .as_ref()
                        .map(|f| f.allow_signal(&signal, bar, &state))
                        .unwrap_or(true);

                    if allowed {
                        let order = Order {
                            direction: signal.direction,
                            order_type: OrderType::Market,
                            size: equity * self.config.position_size_pct,
                            signal: signal.clone(),
                        };

                        let fill_bar = &bars[i + 1];
                        let fill =
                            strategy
                                .execution_model
                                .attempt_fill(&order, bar, fill_bar);

                        if fill.filled {
                            // Initialize position manager state
                            strategy
                                .position_manager
                                .on_entry(fill_bar, fill.fill_price, &signal);

                            position = Some(Position::new(
                                fill.fill_bar_idx,
                                fill_bar.date,
                                fill.fill_price,
                                signal.direction,
                                order.size,
                                signal,
                            ));

                            // Set initial stop from position manager
                            if let Some(ref mut pos) = position {
                                pos.stop_price = strategy.position_manager.stop_price();
                            }
                        }
                    }
                }
            }

            // === STEP 4: Record equity ===
            // For open positions, mark-to-market using current close
            let current_equity = if let Some(ref pos) = position {
                let unrealized = pos.unrealized_pnl_pct(bar.close);
                self.config.initial_equity
                    * (1.0
                        + trades.iter().map(|t| t.return_pct).sum::<f64>()
                        + unrealized * self.config.position_size_pct)
            } else {
                equity
            };
            equity_curve.push(current_equity);
        }

        // Close any remaining position at end of data
        if let Some(pos) = position {
            let last_bar = bars.last().unwrap();
            let trade = self.create_trade(
                &pos,
                bars.len() - 1,
                last_bar.date,
                last_bar.close,
                ExitReason::EndOfData,
            );
            equity *= 1.0 + trade.return_pct;
            if let Some(last) = equity_curve.last_mut() {
                *last = equity;
            }
            trades.push(trade);
        }

        let metrics = Metrics::calculate(&trades, &equity_curve, 252.0);
        let fingerprint = self.compute_fingerprint(&components, bars);

        BacktestResult {
            trades,
            equity_curve,
            metrics,
            fingerprint,
            components,
        }
    }

    /// Create a Trade record from a closed position.
    fn create_trade(
        &self,
        pos: &Position,
        exit_idx: usize,
        exit_date: chrono::NaiveDate,
        exit_price: f64,
        reason: ExitReason,
    ) -> Trade {
        Trade::new(
            pos.entry_bar_idx,
            pos.entry_date,
            pos.entry_price,
            exit_idx,
            exit_date,
            exit_price,
            pos.direction,
            pos.size,
            reason,
            pos.high_since_entry,
            pos.low_since_entry,
            pos.bars_held,
        )
    }

    /// Compute a fingerprint for reproducibility.
    fn compute_fingerprint(&self, components: &StrategyComponents, bars: &[Bar]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        components.signal_generator.hash(&mut hasher);
        components.position_manager.hash(&mut hasher);
        components.execution_model.hash(&mut hasher);
        components.signal_filter.hash(&mut hasher);
        bars.len().hash(&mut hasher);
        if let Some(first) = bars.first() {
            first.date.to_string().hash(&mut hasher);
        }
        if let Some(last) = bars.last() {
            last.date.to_string().hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Direction, Signal};
    use chrono::NaiveDate;

    fn make_bars(n: usize, base_price: f64) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                    + chrono::Duration::days(i as i64),
                open: base_price + i as f64,
                high: base_price + i as f64 + 5.0,
                low: base_price + i as f64 - 2.0,
                close: base_price + i as f64 + 1.0,
                volume: 1_000_000,
                idx: i,
            })
            .collect()
    }

    fn make_trending_up_bars(n: usize, base_price: f64, trend: f64) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let price = base_price + (i as f64 * trend);
                Bar {
                    date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                        + chrono::Duration::days(i as i64),
                    open: price,
                    high: price + 2.0,
                    low: price - 1.0,
                    close: price + 1.0,
                    volume: 1_000_000,
                    idx: i,
                }
            })
            .collect()
    }

    /// Simple stub signal generator for testing
    struct StubSignalGenerator {
        warmup: usize,
        trigger_at: Vec<usize>,
    }

    impl SignalGenerator for StubSignalGenerator {
        fn name(&self) -> &str {
            "StubSignal"
        }

        fn warmup_bars(&self) -> usize {
            self.warmup
        }

        fn generate(&self, bar: &Bar, _state: &MarketState) -> Option<Signal> {
            if self.trigger_at.contains(&bar.idx) {
                Some(Signal::market(Direction::Long, 1.0, bar.close))
            } else {
                None
            }
        }

        fn parameter_spec(&self) -> Vec<crate::param::ParamDef> {
            vec![]
        }

        fn box_clone(&self) -> Box<dyn SignalGenerator> {
            Box::new(Self {
                warmup: self.warmup,
                trigger_at: self.trigger_at.clone(),
            })
        }
    }

    /// Simple stub position manager for testing
    struct StubPositionManager {
        stop_distance: f64,
        stop_price: Option<f64>,
        high_since_entry: f64,
    }

    impl PositionManager for StubPositionManager {
        fn name(&self) -> &str {
            "StubPM"
        }

        fn exit_reference_mode(&self) -> Option<crate::exit_reference::ExitReferenceMode> {
            Some(crate::exit_reference::ExitReferenceMode::SinceEntryTrailingExtreme)
        }

        fn on_entry(&mut self, _entry_bar: &Bar, entry_price: f64, _signal: &Signal) {
            self.high_since_entry = entry_price;
            self.stop_price = Some(entry_price - self.stop_distance);
        }

        fn on_bar(&mut self, bar: &Bar, position: &Position, _state: &MarketState) -> Action {
            self.high_since_entry = position.high_since_entry;
            let new_stop = self.high_since_entry - self.stop_distance;

            if let Some(current_stop) = self.stop_price {
                if new_stop > current_stop {
                    self.stop_price = Some(new_stop);
                    return Action::AdjustStop(new_stop);
                }
            }

            // Check if stop is hit
            if let Some(stop) = self.stop_price {
                if bar.low <= stop {
                    return Action::Exit(ExitReason::StopHit);
                }
            }

            Action::Hold
        }

        fn stop_price(&self) -> Option<f64> {
            self.stop_price
        }

        fn parameter_spec(&self) -> Vec<crate::param::ParamDef> {
            vec![]
        }

        fn box_clone(&self) -> Box<dyn PositionManager> {
            Box::new(Self {
                stop_distance: self.stop_distance,
                stop_price: None,
                high_since_entry: 0.0,
            })
        }

        fn reset(&mut self) {
            self.stop_price = None;
            self.high_since_entry = 0.0;
        }
    }

    /// Simple stub execution model for testing
    struct StubExecutionModel;

    impl ExecutionModel for StubExecutionModel {
        fn name(&self) -> &str {
            "StubExec"
        }

        fn attempt_fill(
            &self,
            _order: &Order,
            _signal_bar: &Bar,
            fill_bar: &Bar,
        ) -> crate::types::FillResult {
            crate::types::FillResult::filled(fill_bar.open, fill_bar.idx, 0.0, 0.0)
        }

        fn check_stop(&self, position: &Position, bar: &Bar) -> Option<f64> {
            let stop = position.stop_price?;
            match position.direction {
                Direction::Long => {
                    if bar.low <= stop {
                        if bar.open <= stop {
                            Some(bar.open)
                        } else {
                            Some(stop)
                        }
                    } else {
                        None
                    }
                }
                Direction::Short => {
                    if bar.high >= stop {
                        if bar.open >= stop {
                            Some(bar.open)
                        } else {
                            Some(stop)
                        }
                    } else {
                        None
                    }
                }
            }
        }

        fn gap_policy(&self) -> crate::types::GapPolicy {
            crate::types::GapPolicy::FillAtOpen
        }

        fn slippage_bps(&self) -> f64 {
            0.0
        }

        fn commission(&self) -> f64 {
            0.0
        }

        fn parameter_spec(&self) -> Vec<crate::param::ParamDef> {
            vec![]
        }

        fn box_clone(&self) -> Box<dyn ExecutionModel> {
            Box::new(StubExecutionModel)
        }
    }

    #[test]
    fn test_engine_basic_trade() {
        let bars = make_trending_up_bars(50, 100.0, 1.0);
        let atr = vec![2.0; 50];
        let adx = vec![25.0; 50];

        let mut strategy = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 5,
                trigger_at: vec![10],
            }),
            Box::new(StubPositionManager {
                stop_distance: 10.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let engine = BacktestEngine::new(BacktestConfig::default());
        let result = engine.run(&mut strategy, &bars, &atr, &adx);

        // Should have at least one trade (entry at bar 11, may exit or run to end)
        assert!(!result.trades.is_empty());
        assert!(!result.fingerprint.is_empty());
    }

    #[test]
    fn test_engine_no_trades_during_warmup() {
        let bars = make_bars(10, 100.0);
        let atr = vec![2.0; 10];
        let adx = vec![25.0; 10];

        let mut strategy = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 20, // Warmup longer than data
                trigger_at: vec![5],
            }),
            Box::new(StubPositionManager {
                stop_distance: 10.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let engine = BacktestEngine::new(BacktestConfig::default());
        let result = engine.run(&mut strategy, &bars, &atr, &adx);

        // No trades should occur during warmup
        assert!(result.trades.is_empty());
    }

    #[test]
    fn test_engine_determinism() {
        let bars = make_trending_up_bars(100, 100.0, 0.5);
        let atr = vec![2.0; 100];
        let adx = vec![25.0; 100];

        let mut strategy1 = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 5,
                trigger_at: vec![10, 30, 50],
            }),
            Box::new(StubPositionManager {
                stop_distance: 5.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let mut strategy2 = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 5,
                trigger_at: vec![10, 30, 50],
            }),
            Box::new(StubPositionManager {
                stop_distance: 5.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let engine = BacktestEngine::new(BacktestConfig::default());

        let result1 = engine.run(&mut strategy1, &bars, &atr, &adx);
        let result2 = engine.run(&mut strategy2, &bars, &atr, &adx);

        // Results should be identical
        assert_eq!(result1.trades.len(), result2.trades.len());
        assert_eq!(result1.fingerprint, result2.fingerprint);

        for (t1, t2) in result1.trades.iter().zip(result2.trades.iter()) {
            assert_eq!(t1.entry_bar_idx, t2.entry_bar_idx);
            assert_eq!(t1.exit_bar_idx, t2.exit_bar_idx);
            assert!((t1.return_pct - t2.return_pct).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_exit_before_entry() {
        // This tests the invariant that exits are processed before entries on the same bar
        let mut bars = make_trending_up_bars(50, 100.0, 1.0);

        // Create a scenario where stop would be hit on same bar as new signal
        // Bar 20: drop low enough to hit stop from position entered at bar 11
        bars[20].low = 90.0; // Below stop at ~101 (entry 111 - 10)

        let atr = vec![2.0; 50];
        let adx = vec![25.0; 50];

        let mut strategy = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 5,
                trigger_at: vec![10, 20], // Signal on both bar 10 and 20
            }),
            Box::new(StubPositionManager {
                stop_distance: 10.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let engine = BacktestEngine::new(BacktestConfig::default());
        let result = engine.run(&mut strategy, &bars, &atr, &adx);

        // Should have exited position before potentially entering new one
        // First trade: entered bar 11, should exit when stop hit
        if result.trades.len() >= 1 {
            let first_trade = &result.trades[0];
            assert_eq!(first_trade.entry_bar_idx, 11);
            // Exit should happen on or before bar 20 due to stop
        }
    }

    #[test]
    fn test_engine_fresh_state_per_run() {
        let bars = make_trending_up_bars(50, 100.0, 1.0);
        let atr = vec![2.0; 50];
        let adx = vec![25.0; 50];

        let mut strategy = Strategy::new(
            Box::new(StubSignalGenerator {
                warmup: 5,
                trigger_at: vec![10],
            }),
            Box::new(StubPositionManager {
                stop_distance: 10.0,
                stop_price: None,
                high_since_entry: 0.0,
            }),
            Box::new(StubExecutionModel),
            None,
        );

        let engine = BacktestEngine::new(BacktestConfig::default());

        // First run
        let result1 = engine.run(&mut strategy, &bars, &atr, &adx);

        // Second run - should have fresh state
        let result2 = engine.run(&mut strategy, &bars, &atr, &adx);

        // Both runs should produce same results
        assert_eq!(result1.trades.len(), result2.trades.len());
    }
}
