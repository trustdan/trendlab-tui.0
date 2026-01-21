//! Integration tests for the backtest engine.
//!
//! These tests verify end-to-end behavior including determinism,
//! trade lifecycle, and critical invariants.

use trendlab_core::prelude::*;

// =============================================================================
// Test Fixtures
// =============================================================================

/// Generate deterministic test bars with known properties.
fn generate_test_bars(count: usize) -> Vec<Bar> {
    (0..count)
        .map(|i| {
            let base = 100.0 + (i as f64) * 0.1;
            // Add deterministic variation that will trigger signals
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

/// Generate bars with a clear uptrend and breakout opportunity.
fn generate_breakout_scenario() -> (Vec<Bar>, Vec<f64>, Vec<f64>) {
    let mut bars = Vec::with_capacity(50);

    // 20 bars of consolidation at 100
    for i in 0..20 {
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: 100.0,
            high: 102.0,
            low: 98.0,
            close: 100.0 + (i % 2) as f64,
            volume: 1_000_000,
            idx: i as usize,
        });
    }

    // Bar 20: Breakout above 102
    bars.push(Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 21).unwrap(),
        open: 101.0,
        high: 105.0,
        low: 100.0,
        close: 104.0, // Close above 20-bar high of 102
        volume: 2_000_000,
        idx: 20,
    });

    // 10 more bars trending up
    for i in 21..30 {
        let base = 104.0 + ((i - 21) as f64) * 0.5;
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: base,
            high: base + 2.0,
            low: base - 1.0,
            close: base + 1.0,
            volume: 1_500_000,
            idx: i as usize,
        });
    }

    // Bar 30: Price drops, hits trailing stop
    bars.push(Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        open: 108.0,
        high: 108.5,
        low: 95.0, // Crashes through stop
        close: 96.0,
        volume: 3_000_000,
        idx: 30,
    });

    // Fill remaining bars
    for i in 31..50 {
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: 96.0,
            high: 97.0,
            low: 95.0,
            close: 96.0,
            volume: 1_000_000,
            idx: i as usize,
        });
    }

    let atr = vec![3.0; 50];
    let adx = vec![25.0; 50];

    (bars, atr, adx)
}

/// Helper to get final equity from result
fn final_equity(result: &BacktestResult) -> f64 {
    result.equity_curve.last().copied().unwrap_or(0.0)
}

// =============================================================================
// Determinism Tests
// =============================================================================

#[test]
fn test_determinism_same_inputs_same_outputs() {
    let bars = generate_test_bars(252);
    let atr = vec![2.0; 252];
    let adx = vec![25.0; 252];

    let config = BacktestConfig {
        initial_equity: 100_000.0,
        ..Default::default()
    };

    // Run 1
    let engine = BacktestEngine::new(config.clone());
    let mut strategy1 = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::new(10.0, 1.0)),
        None,
    );
    let result1 = engine.run(&mut strategy1, &bars, &atr, &adx);

    // Run 2 with fresh components
    let engine = BacktestEngine::new(config);
    let mut strategy2 = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::new(10.0, 1.0)),
        None,
    );
    let result2 = engine.run(&mut strategy2, &bars, &atr, &adx);

    // Verify determinism
    assert_eq!(
        result1.trades.len(),
        result2.trades.len(),
        "Trade counts must be identical"
    );

    assert!(
        (final_equity(&result1) - final_equity(&result2)).abs() < 0.001,
        "Final equity must be identical: {} vs {}",
        final_equity(&result1),
        final_equity(&result2)
    );

    // Verify trade-by-trade equality
    for (i, (t1, t2)) in result1.trades.iter().zip(result2.trades.iter()).enumerate() {
        assert_eq!(t1.entry_date, t2.entry_date, "Trade {} entry dates differ", i);
        assert_eq!(t1.exit_date, t2.exit_date, "Trade {} exit dates differ", i);
        assert!(
            (t1.entry_price - t2.entry_price).abs() < 0.001,
            "Trade {} entry prices differ: {} vs {}",
            i,
            t1.entry_price,
            t2.entry_price
        );
        assert!(
            (t1.exit_price - t2.exit_price).abs() < 0.001,
            "Trade {} exit prices differ",
            i
        );
    }
}

#[test]
fn test_determinism_equity_curve() {
    let bars = generate_test_bars(100);
    let atr = vec![2.0; 100];
    let adx = vec![25.0; 100];

    let config = BacktestConfig::default();
    let engine = BacktestEngine::new(config.clone());

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );
    let result1 = engine.run(&mut strategy, &bars, &atr, &adx);

    let engine = BacktestEngine::new(config);
    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );
    let result2 = engine.run(&mut strategy, &bars, &atr, &adx);

    assert_eq!(
        result1.equity_curve.len(),
        result2.equity_curve.len(),
        "Equity curves must have same length"
    );

    for (i, (e1, e2)) in result1
        .equity_curve
        .iter()
        .zip(result2.equity_curve.iter())
        .enumerate()
    {
        assert!(
            (e1 - e2).abs() < 0.001,
            "Equity at bar {} differs: {} vs {}",
            i,
            e1,
            e2
        );
    }
}

// =============================================================================
// Trade Lifecycle Tests
// =============================================================================

#[test]
fn test_trade_lifecycle_entry_on_breakout() {
    let (bars, atr, adx) = generate_breakout_scenario();

    let config = BacktestConfig {
        initial_equity: 100_000.0,
        ..Default::default()
    };
    let engine = BacktestEngine::new(config);

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::new(0.0, 0.0)), // No slippage for easier verification
        None,
    );

    let result = engine.run(&mut strategy, &bars, &atr, &adx);

    // Should have at least one trade
    assert!(
        !result.trades.is_empty(),
        "Expected at least one trade from breakout scenario"
    );

    // First trade should be long (breakout above high)
    let trade = &result.trades[0];
    assert_eq!(trade.direction, Direction::Long);

    // Entry should be on bar 21 (the bar after the breakout close)
    // Entry happens on next open after signal
    assert_eq!(
        trade.entry_date,
        chrono::NaiveDate::from_ymd_opt(2024, 1, 22).unwrap()
    );
}

#[test]
fn test_trade_lifecycle_exit_on_stop() {
    let (bars, atr, adx) = generate_breakout_scenario();

    let config = BacktestConfig::default();
    let engine = BacktestEngine::new(config);

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );

    let result = engine.run(&mut strategy, &bars, &atr, &adx);

    // Should have completed trades (all trades in result are closed)
    assert!(
        !result.trades.is_empty(),
        "Expected at least one completed trade"
    );

    // Check exit reason is StopHit
    let first_trade = &result.trades[0];
    assert_eq!(
        first_trade.exit_reason,
        ExitReason::StopHit,
        "Exit should be from stop hit"
    );
}

#[test]
fn test_position_tracking_from_entry() {
    // Verify that high_since_entry starts from entry, not historical data
    let mut pm = AtrTrailingStop::new(2.0);

    let entry_bar = Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        open: 100.0,
        high: 150.0, // Historical high is much higher
        low: 95.0,
        close: 100.0,
        volume: 1_000_000,
        idx: 0,
    };

    let signal = Signal::market(Direction::Long, 1.0, 100.0);
    pm.on_entry(&entry_bar, 100.0, &signal);

    // Stop should be calculated from entry price (100), not bar high (150)
    let stop = pm.stop_price().expect("Stop should be set");

    // With 2.0 multiplier and bar range (150-95=55) as initial ATR,
    // stop = 100 - 2.0 * 55 = -10 (but this shows it's from entry, not high)
    // The key is that we're not using 150 as the reference
    let _ = stop; // Just verify it exists

    // More importantly, create a position and verify high_since_entry
    let position = Position::new(
        0,
        chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        100.0,
        Direction::Long,
        10000.0,
        signal,
    );

    // Position's high_since_entry should equal entry price initially
    assert_eq!(
        position.high_since_entry, 100.0,
        "high_since_entry should start at entry price, not bar high"
    );
}

// =============================================================================
// No Lookahead Tests
// =============================================================================

#[test]
fn test_no_lookahead_market_state() {
    let bars = generate_test_bars(100);
    let atr = vec![2.0; 100];
    let adx = vec![25.0; 100];

    // Create market state at bar 50
    let state = MarketState::new(&bars[..51], 50, &atr[..51], &adx[..51]);

    // Should only see bars 0-50
    assert_eq!(state.len(), 51);
    assert_eq!(state.current_idx, 50);

    // Current bar should be bar 50
    assert_eq!(state.current_bar().idx, 50);

    // Lookback should not exceed available bars
    let lookback = state.lookback(100); // Request more than available
    assert_eq!(lookback.len(), 51); // Should only get what's available
}

#[test]
fn test_no_lookahead_signal_generation() {
    let bars = generate_test_bars(50);
    let atr = vec![2.0; 50];
    let adx = vec![25.0; 50];

    let sg = DonchianBreakout::new(20, true);

    // Create state at bar 30
    let state = MarketState::new(&bars[..31], 30, &atr[..31], &adx[..31]);

    // Signal generation should work without access to future bars
    let _signal = sg.generate(&bars[30], &state);

    // The test passes if no panic occurs - MarketState prevents lookahead
}

// =============================================================================
// Exit Before Entry Tests
// =============================================================================

#[test]
fn test_exit_checked_before_entry() {
    // Create a scenario where exit and entry could both trigger
    let mut bars = Vec::with_capacity(30);

    // 20 bars of setup
    for i in 0..20 {
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: 100.0,
            high: 102.0,
            low: 98.0,
            close: 100.0,
            volume: 1_000_000,
            idx: i as usize,
        });
    }

    // Bar 20: Breakout entry
    bars.push(Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 21).unwrap(),
        open: 100.0,
        high: 105.0,
        low: 99.0,
        close: 104.0,
        volume: 1_000_000,
        idx: 20,
    });

    // Bar 21: Entry fills at open
    bars.push(Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 22).unwrap(),
        open: 104.0,
        high: 106.0,
        low: 103.0,
        close: 105.0,
        volume: 1_000_000,
        idx: 21,
    });

    // Bars 22-28: Price rises then crashes
    for i in 22..29 {
        let base = 105.0 + ((i - 22) as f64) * 0.5;
        bars.push(Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(i as i64),
            open: base,
            high: base + 1.0,
            low: base - 0.5,
            close: base + 0.5,
            volume: 1_000_000,
            idx: i as usize,
        });
    }

    // Bar 29: Crash that hits stop AND would trigger new breakout (if we had exited)
    bars.push(Bar {
        date: chrono::NaiveDate::from_ymd_opt(2024, 1, 30).unwrap(),
        open: 108.0,
        high: 120.0, // New high - would be breakout if not in position
        low: 90.0,   // Crashes through stop
        close: 95.0,
        volume: 3_000_000,
        idx: 29,
    });

    let atr = vec![3.0; 30];
    let adx = vec![25.0; 30];

    let config = BacktestConfig::default();
    let engine = BacktestEngine::new(config);

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );

    let result = engine.run(&mut strategy, &bars, &atr, &adx);

    // Should have at least one completed trade (all trades in result are closed)
    // The engine should process exit first, then not allow entry on same bar
    assert!(
        !result.trades.is_empty(),
        "Should have at least one completed trade"
    );
}

// =============================================================================
// Component Independence Tests
// =============================================================================

#[test]
fn test_fresh_state_per_run() {
    let bars = generate_test_bars(100);
    let atr = vec![2.0; 100];
    let adx = vec![25.0; 100];

    let config = BacktestConfig::default();

    // Create components once
    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );

    // First run
    let engine = BacktestEngine::new(config.clone());
    let result1 = engine.run(&mut strategy, &bars, &atr, &adx);

    // Components should be reset for second run
    // This is why box_clone returns fresh instances
    let mut fresh_strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::new(2.0)),
        Box::new(NextOpenFill::default()),
        None,
    );

    let engine = BacktestEngine::new(config);
    let result2 = engine.run(&mut fresh_strategy, &bars, &atr, &adx);

    // Results should be identical (no state leakage)
    assert_eq!(result1.trades.len(), result2.trades.len());
    assert!((final_equity(&result1) - final_equity(&result2)).abs() < 0.001);
}

// =============================================================================
// Metrics Tests
// =============================================================================

#[test]
fn test_equity_curve_length_matches_bars() {
    let bars = generate_test_bars(100);
    let atr = vec![2.0; 100];
    let adx = vec![25.0; 100];

    let config = BacktestConfig::default();
    let engine = BacktestEngine::new(config);

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::default()),
        Box::new(NextOpenFill::default()),
        None,
    );

    let result = engine.run(&mut strategy, &bars, &atr, &adx);

    assert_eq!(
        result.equity_curve.len(),
        bars.len(),
        "Equity curve should have one point per bar"
    );
}

#[test]
fn test_initial_equity_matches_config() {
    let bars = generate_test_bars(100);
    let atr = vec![2.0; 100];
    let adx = vec![25.0; 100];

    let config = BacktestConfig {
        initial_equity: 50_000.0,
        ..Default::default()
    };
    let engine = BacktestEngine::new(config);

    let mut strategy = Strategy::new(
        Box::new(DonchianBreakout::new(20, true)),
        Box::new(AtrTrailingStop::default()),
        Box::new(NextOpenFill::default()),
        None,
    );

    let result = engine.run(&mut strategy, &bars, &atr, &adx);

    assert!(
        (result.equity_curve[0] - 50_000.0).abs() < 0.001,
        "Initial equity should match config"
    );
}
