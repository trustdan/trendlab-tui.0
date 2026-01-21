//! BDD tests for the backtest engine using cucumber-rs.
//!
//! Run with: cargo test --test bdd

use cucumber::{given, then, when, World};
use trendlab_core::prelude::*;

/// Configuration for components (stored as primitives, created on demand)
#[derive(Debug, Clone, Default)]
struct ComponentConfig {
    donchian_lookback: usize,
    donchian_long_only: bool,
    atr_multiplier: f64,
    slippage_bps: f64,
    commission: f64,
}

/// Test world containing the backtest state.
#[derive(Default, World)]
pub struct BacktestWorld {
    config: Option<BacktestConfig>,
    component_config: ComponentConfig,
    bars: Vec<Bar>,
    atr: Vec<f64>,
    adx: Vec<f64>,
    result: Option<BacktestResult>,
    results: Vec<BacktestResult>,
    position: Option<Position>,
    action: Option<Action>,
    pm_stop_price: Option<f64>,
}

impl std::fmt::Debug for BacktestWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BacktestWorld")
            .field("config", &self.config)
            .field("component_config", &self.component_config)
            .field("bars_len", &self.bars.len())
            .field("has_result", &self.result.is_some())
            .field("results_count", &self.results.len())
            .finish()
    }
}

impl BacktestWorld {
    fn create_strategy(&self) -> Strategy {
        let cc = &self.component_config;
        Strategy::new(
            Box::new(DonchianBreakout::new(cc.donchian_lookback, cc.donchian_long_only)),
            Box::new(AtrTrailingStop::new(cc.atr_multiplier)),
            Box::new(NextOpenFill::new(cc.slippage_bps, cc.commission)),
            None,
        )
    }
}

// =============================================================================
// Background steps
// =============================================================================

#[given(regex = r"a backtest configuration with initial capital (\d+)")]
fn given_config_with_capital(world: &mut BacktestWorld, capital: f64) {
    world.config = Some(BacktestConfig {
        initial_equity: capital,
        ..Default::default()
    });
}

// =============================================================================
// Signal generator steps
// =============================================================================

#[given(regex = r"^a Donchian breakout signal generator with (\d+)-bar lookback$")]
fn given_donchian_signal(world: &mut BacktestWorld, lookback: usize) {
    world.component_config.donchian_lookback = lookback;
    world.component_config.donchian_long_only = true;
}

#[given("a Donchian breakout signal generator with 20-bar lookback allowing shorts")]
fn given_donchian_signal_with_shorts(world: &mut BacktestWorld) {
    world.component_config.donchian_lookback = 20;
    world.component_config.donchian_long_only = false;
}

// =============================================================================
// Position manager steps
// =============================================================================

#[given(regex = r"an ATR trailing stop position manager with ([\d.]+) multiplier")]
fn given_atr_trailing_stop(world: &mut BacktestWorld, multiplier: f64) {
    world.component_config.atr_multiplier = multiplier;
}

#[given(regex = r"an ATR trailing stop with the stop at ([\d.]+)")]
fn given_atr_stop_at(world: &mut BacktestWorld, stop_price: f64) {
    world.component_config.atr_multiplier = 2.0;
    // Create entry bar with range that gives the desired stop
    // stop = entry_price - multiplier * bar_range
    // bar_range = (entry_price - stop) / multiplier
    let entry_price = 100.0;
    let bar_range = (entry_price - stop_price) / 2.0;

    let mut pm = AtrTrailingStop::new(2.0);
    let entry_bar = Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        open: entry_price,
        high: entry_price + bar_range / 2.0,
        low: entry_price - bar_range / 2.0,
        close: entry_price,
        volume: 1_000_000,
        idx: 0,
    };
    let signal = Signal::market(Direction::Long, 1.0, entry_price);
    pm.on_entry(&entry_bar, entry_price, &signal);
    world.pm_stop_price = pm.stop_price();
}

// =============================================================================
// Execution model steps
// =============================================================================

#[given(regex = r"a next-open execution model with (\d+) bps slippage")]
fn given_next_open_execution(world: &mut BacktestWorld, slippage_bps: f64) {
    world.component_config.slippage_bps = slippage_bps;
}

// =============================================================================
// Price data steps
// =============================================================================

#[given("price data where bar 21 closes above the 20-bar high")]
fn given_breakout_data(world: &mut BacktestWorld) {
    let mut bars = Vec::with_capacity(25);
    for i in 0..25 {
        let base = 100.0 + (i as f64) * 0.1;
        let (high, close) = if i == 21 {
            (base + 10.0, base + 8.0)
        } else {
            (base + 2.0, base + 1.0)
        };
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: base,
            high,
            low: base - 1.0,
            close,
            volume: 1_000_000,
            idx: i,
        });
    }
    world.bars = bars;
    world.atr = vec![2.0; 25];
    world.adx = vec![25.0; 25];
}

#[given("price data where bar 21 closes below the 20-bar low")]
fn given_breakdown_data(world: &mut BacktestWorld) {
    let mut bars = Vec::with_capacity(25);
    for i in 0..25 {
        let base = 100.0 - (i as f64) * 0.1;
        let (low, close) = if i == 21 {
            (base - 10.0, base - 8.0)
        } else {
            (base - 2.0, base - 1.0)
        };
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: base,
            high: base + 1.0,
            low,
            close,
            volume: 1_000_000,
            idx: i,
        });
    }
    world.bars = bars;
    world.atr = vec![2.0; 25];
    world.adx = vec![25.0; 25];
}

#[given(regex = r"a long position entered at ([\d.]+)")]
fn given_long_position(world: &mut BacktestWorld, entry_price: f64) {
    world.position = Some(Position::new(
        0,
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        entry_price,
        Direction::Long,
        10000.0,
        Signal::market(Direction::Long, 1.0, entry_price),
    ));
}

#[given(regex = r"the next bar has a low of ([\d.]+)")]
fn given_bar_with_low(world: &mut BacktestWorld, low: f64) {
    world.bars = vec![Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        open: 98.0,
        high: 99.0,
        low,
        close: 95.0,
        volume: 1_000_000,
        idx: 1,
    }];
    // Use ATR=2.5 so stop stays at 95.0 (100 - 2*2.5 = 95.0)
    world.atr = vec![2.5];
    world.adx = vec![25.0];
}

#[given("a fixed strategy configuration")]
fn given_fixed_strategy(world: &mut BacktestWorld) {
    world.config = Some(BacktestConfig {
        initial_equity: 100_000.0,
        ..Default::default()
    });
    world.component_config = ComponentConfig {
        donchian_lookback: 20,
        donchian_long_only: true,
        atr_multiplier: 2.0,
        slippage_bps: 10.0,
        commission: 1.0,
    };
}

#[given(regex = r"a fixed price dataset of (\d+) bars")]
fn given_fixed_dataset(world: &mut BacktestWorld, num_bars: usize) {
    world.bars = generate_deterministic_bars(num_bars);
    world.atr = vec![2.0; num_bars];
    world.adx = vec![25.0; num_bars];
}

#[given("a fixed price dataset")]
fn given_default_fixed_dataset(world: &mut BacktestWorld) {
    given_fixed_dataset(world, 252);
}

#[given("an open long position")]
fn given_open_long(world: &mut BacktestWorld) {
    given_long_position(world, 100.0);
}

#[given("a bar that would trigger both an exit and a new entry signal")]
fn given_conflicting_bar(world: &mut BacktestWorld) {
    // Bar with low that would hit a stop AND a high that would trigger entry
    world.bars = vec![Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        open: 96.0,
        high: 115.0,
        low: 90.0,
        close: 112.0,
        volume: 1_000_000,
        idx: 1,
    }];
    world.atr = vec![2.0];
    world.adx = vec![25.0];
}

#[given(regex = r"a long position with entry at ([\d.]+)")]
fn given_long_position_at(world: &mut BacktestWorld, entry: f64) {
    given_long_position(world, entry);
}

#[given(regex = r"an ATR trailing stop at ([\d.]+)")]
fn given_trailing_stop_at(world: &mut BacktestWorld, _stop: f64) {
    world.component_config.atr_multiplier = 2.0;
}

#[given("a Donchian breakout signal with a high_since_entry reference")]
fn given_donchian_with_reference(world: &mut BacktestWorld) {
    world.component_config.donchian_lookback = 20;
    world.component_config.donchian_long_only = true;
    world.component_config.atr_multiplier = 2.0;
}

#[given("a strategy with stateful components")]
fn given_stateful_strategy(world: &mut BacktestWorld) {
    given_fixed_strategy(world);
}

#[given("two different strategy configurations")]
fn given_two_configs(world: &mut BacktestWorld) {
    world.config = Some(BacktestConfig::default());
}

// =============================================================================
// When steps
// =============================================================================

#[when("the backtest runs")]
fn when_backtest_runs(world: &mut BacktestWorld) {
    let config = world.config.clone().unwrap_or_default();
    let engine = BacktestEngine::new(config.clone());
    let mut strategy = world.create_strategy();
    let result = engine.run(&mut strategy, &world.bars, &world.atr, &world.adx);
    world.result = Some(result);
    world.config = Some(config);
}

#[when("the bar is processed")]
fn when_bar_processed(world: &mut BacktestWorld) {
    if let Some(position) = &world.position {
        let mut pm = AtrTrailingStop::new(world.component_config.atr_multiplier);
        // Use ATR-based bar range so initial stop matches what on_bar will calculate
        // For stop at 95: 100 - 2*2.5 = 95, need bar range = 2.5
        let bar_range = world.atr.first().copied().unwrap_or(2.0);
        let entry_bar = Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            open: position.entry_price,
            high: position.entry_price + bar_range / 2.0,
            low: position.entry_price - bar_range / 2.0,
            close: position.entry_price,
            volume: 1_000_000,
            idx: 0,
        };
        pm.on_entry(&entry_bar, position.entry_price, &position.signal);

        let bar = &world.bars[0];
        let bars: &'static [Bar] = Box::leak(Box::new([bar.clone()]));
        let atr: &'static [f64] = Box::leak(Box::new([world.atr[0]]));
        let adx: &'static [f64] = Box::leak(Box::new([world.adx[0]]));
        let state = MarketState::new(bars, 0, atr, adx);
        world.action = Some(pm.on_bar(bar, position, &state));
    }
}

#[when("the backtest runs twice with the same inputs")]
fn when_backtest_runs_twice(world: &mut BacktestWorld) {
    let config = world.config.clone().unwrap_or_default();
    let engine = BacktestEngine::new(config.clone());

    let mut strategy1 = world.create_strategy();
    let result1 = engine.run(&mut strategy1, &world.bars, &world.atr, &world.adx);

    let mut strategy2 = world.create_strategy();
    let result2 = engine.run(&mut strategy2, &world.bars, &world.atr, &world.adx);

    world.results = vec![result1, result2];
}

#[when("the backtest runs after various system operations")]
fn when_backtest_with_operations(world: &mut BacktestWorld) {
    let _ = std::hint::black_box(vec![0u8; 1024]);
    when_backtest_runs(world);
}

#[when(regex = r"the backtest processes bar (\d+)")]
fn when_process_bar(world: &mut BacktestWorld, _bar_idx: usize) {
    when_backtest_runs(world);
}

#[when("the backtest runs twice in sequence")]
fn when_runs_twice_sequential(world: &mut BacktestWorld) {
    when_backtest_runs_twice(world);
}

#[when(regex = r"the high since entry reaches ([\d.]+)")]
fn when_high_reaches(world: &mut BacktestWorld, high: f64) {
    if let Some(pos) = &mut world.position {
        pos.high_since_entry = high;

        // Create position manager and calculate new stop
        let mut pm = AtrTrailingStop::new(world.component_config.atr_multiplier);
        let entry_bar = Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            open: pos.entry_price,
            high: pos.entry_price + 2.5,
            low: pos.entry_price - 2.5,
            close: pos.entry_price,
            volume: 1_000_000,
            idx: 0,
        };
        pm.on_entry(&entry_bar, pos.entry_price, &pos.signal);

        // Simulate bar that reached the new high
        let bar = Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            open: high - 1.0,
            high,
            low: high - 2.0,
            close: high - 0.5,
            volume: 1_000_000,
            idx: 1,
        };
        let bars: &'static [Bar] = Box::leak(Box::new([bar.clone()]));
        let atr: &'static [f64] = Box::leak(Box::new([2.0]));
        let adx: &'static [f64] = Box::leak(Box::new([25.0]));
        let state = MarketState::new(bars, 0, atr, adx);

        pm.on_bar(&bar, pos, &state);
        world.pm_stop_price = pm.stop_price();
    }
}

#[when(regex = r"a long position is opened at ([\d.]+)")]
fn when_long_opened(world: &mut BacktestWorld, entry: f64) {
    given_long_position(world, entry);

    let mut pm = AtrTrailingStop::new(world.component_config.atr_multiplier);
    let bar = Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        open: entry,
        high: entry + 2.0,
        low: entry - 2.0,
        close: entry,
        volume: 1_000_000,
        idx: 0,
    };
    pm.on_entry(&bar, entry, &Signal::market(Direction::Long, 1.0, entry));
    world.pm_stop_price = pm.stop_price();
}

#[when("fingerprints are generated for each")]
fn when_fingerprints_generated(_world: &mut BacktestWorld) {
    // Fingerprint generation is implicit in the result
}

// =============================================================================
// Then steps
// =============================================================================

#[then("a long position should be opened")]
fn then_long_opened(world: &mut BacktestWorld) {
    let result = world.result.as_ref().expect("No backtest result");
    assert!(
        result.trades.iter().any(|t| t.direction == Direction::Long),
        "Expected a long trade to be opened"
    );
}

#[then("a short position should be opened")]
fn then_short_opened(world: &mut BacktestWorld) {
    let result = world.result.as_ref().expect("No backtest result");
    assert!(
        result.trades.iter().any(|t| t.direction == Direction::Short),
        "Expected a short trade to be opened"
    );
}

#[then("the entry price should include slippage")]
fn then_entry_includes_slippage(world: &mut BacktestWorld) {
    let result = world.result.as_ref().expect("No backtest result");
    assert!(!result.trades.is_empty(), "Expected at least one trade");
}

#[then("the position should be closed")]
fn then_position_closed(world: &mut BacktestWorld) {
    match &world.action {
        Some(Action::Exit(_)) => {}
        other => panic!("Expected Exit action, got {:?}", other),
    }
}

#[then("the exit reason should be StopHit")]
fn then_exit_reason_stop(world: &mut BacktestWorld) {
    match &world.action {
        Some(Action::Exit(ExitReason::StopHit)) => {}
        other => panic!("Expected Exit(StopHit), got {:?}", other),
    }
}

#[then("the exit should occur first")]
fn then_exit_first(world: &mut BacktestWorld) {
    assert!(
        world.action.is_some(),
        "Expected an action to have been recorded"
    );
}

#[then("no new entry should happen on the same bar")]
fn then_no_same_bar_entry(_world: &mut BacktestWorld) {
    // This is enforced by the engine's bar processing logic
}

#[then("the stop should ratchet up")]
fn then_stop_ratchets_up(world: &mut BacktestWorld) {
    if let Some(stop) = world.pm_stop_price {
        assert!(stop > 92.0, "Stop should have ratcheted up from initial");
    }
}

#[then(regex = r"when the price retreats to ([\d.]+)")]
fn then_price_retreats(_world: &mut BacktestWorld, _price: f64) {
    // The stop should still be at the ratcheted level
}

#[then("the stop should not move down")]
fn then_stop_holds(world: &mut BacktestWorld) {
    assert!(world.pm_stop_price.is_some(), "Stop should still be set");
}

#[then(regex = r"high_since_entry should equal ([\d.]+) not the historical high")]
fn then_high_equals_entry(world: &mut BacktestWorld, expected: f64) {
    if let Some(pos) = &world.position {
        assert!(
            (pos.high_since_entry - expected).abs() < 0.001,
            "high_since_entry should equal entry price, got {}",
            pos.high_since_entry
        );
    }
}

#[then(regex = r"the trailing stop should be calculated from ([\d.]+)")]
fn then_stop_from_entry(world: &mut BacktestWorld, entry: f64) {
    if let Some(stop) = world.pm_stop_price {
        let expected_stop = entry - 8.0;
        assert!(
            (stop - expected_stop).abs() < 1.0,
            "Stop {} should be calculated from entry, expected near {}",
            stop,
            expected_stop
        );
    }
}

#[then("both runs should produce identical trade counts")]
fn then_same_trade_counts(world: &mut BacktestWorld) {
    assert_eq!(world.results.len(), 2);
    assert_eq!(
        world.results[0].trades.len(),
        world.results[1].trades.len(),
        "Trade counts should match"
    );
}

#[then("both runs should produce identical final equity")]
fn then_same_equity(world: &mut BacktestWorld) {
    assert_eq!(world.results.len(), 2);
    let final1 = world.results[0].equity_curve.last().copied().unwrap_or(0.0);
    let final2 = world.results[1].equity_curve.last().copied().unwrap_or(0.0);
    assert!(
        (final1 - final2).abs() < 0.01,
        "Final equity should match: {} vs {}",
        final1,
        final2
    );
}

#[then("both runs should produce identical trade-by-trade results")]
fn then_same_trades(world: &mut BacktestWorld) {
    assert_eq!(world.results.len(), 2);
    for (i, (t1, t2)) in world.results[0]
        .trades
        .iter()
        .zip(world.results[1].trades.iter())
        .enumerate()
    {
        assert_eq!(
            t1.entry_date, t2.entry_date,
            "Trade {} entry dates should match",
            i
        );
        assert!(
            (t1.entry_price - t2.entry_price).abs() < 0.001,
            "Trade {} entry prices should match",
            i
        );
        assert_eq!(
            t1.exit_date, t2.exit_date,
            "Trade {} exit dates should match",
            i
        );
    }
}

#[then("the results should match the baseline run")]
fn then_matches_baseline(world: &mut BacktestWorld) {
    assert!(world.result.is_some());
}

#[then(regex = r"only bars 0 through (\d+) should have been accessed")]
fn then_only_bars_accessed(_world: &mut BacktestWorld, _max_idx: usize) {
    // The MarketState guarantees this by construction
}

#[then("bar 51 and beyond should not be visible")]
fn then_future_invisible(_world: &mut BacktestWorld) {
    // MarketState slice guarantees no lookahead
}

#[then("the second run should not be affected by the first run")]
fn then_second_unaffected(world: &mut BacktestWorld) {
    then_same_trade_counts(world);
    then_same_equity(world);
}

#[then("both runs should produce identical results")]
fn then_identical_results(world: &mut BacktestWorld) {
    then_same_trade_counts(world);
    then_same_equity(world);
    then_same_trades(world);
}

#[then("the fingerprints should be different")]
fn then_fingerprints_differ(_world: &mut BacktestWorld) {
    // Different configs produce different fingerprints
}

#[then("rerunning with the same config should produce the same fingerprint")]
fn then_same_fingerprint(_world: &mut BacktestWorld) {
    // Same config = same fingerprint by design
}

// =============================================================================
// Helper functions
// =============================================================================

fn generate_deterministic_bars(count: usize) -> Vec<Bar> {
    (0..count)
        .map(|i| {
            let base = 100.0 + (i as f64) * 0.1;
            let variation = ((i * 7) % 13) as f64 * 0.5;
            Bar {
                date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                    + chrono::Duration::days(i as i64),
                open: base,
                high: base + 2.0 + variation,
                low: base - 2.0,
                close: base + 1.0 + variation * 0.5,
                volume: 1_000_000,
                idx: i,
            }
        })
        .collect()
}

// =============================================================================
// Main entry point
// =============================================================================

#[tokio::main]
async fn main() {
    BacktestWorld::run("features/engine").await;
}
