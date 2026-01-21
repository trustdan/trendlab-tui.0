//! Property-based tests for trendlab-core invariants.

use chrono::NaiveDate;
use proptest::prelude::*;
use proptest::strategy::Strategy as PropStrategy;
use trendlab_core::prelude::*;

/// Generate a valid bar with high >= low constraint
fn valid_bar_strategy() -> impl PropStrategy<Value = Bar> {
    (
        1.0..1000.0f64,  // open
        1.0..1000.0f64,  // close
        0..1000usize,    // idx
    )
        .prop_map(|(open, close, idx)| {
            let high = open.max(close) * 1.05;
            let low = open.min(close) * 0.95;
            Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open,
                high,
                low,
                close,
                volume: 1_000_000,
                idx,
            }
        })
}

proptest! {
    /// Bar high is always >= low
    #[test]
    fn prop_bar_high_gte_low(bar in valid_bar_strategy()) {
        prop_assert!(bar.high >= bar.low);
        prop_assert!(bar.high >= bar.open);
        prop_assert!(bar.high >= bar.close);
        prop_assert!(bar.low <= bar.open);
        prop_assert!(bar.low <= bar.close);
    }

    /// True range is always non-negative
    #[test]
    fn prop_true_range_non_negative(
        bar in valid_bar_strategy(),
        prev_close in prop::option::of(50.0..200.0f64),
    ) {
        let tr = bar.true_range(prev_close);
        prop_assert!(tr >= 0.0);
    }

    /// MarketState never exposes future bars
    #[test]
    fn prop_market_state_no_future(
        num_bars in 10..100usize,
        current_idx in 0..10usize,
    ) {
        let current_idx = current_idx.min(num_bars - 1);
        let bars: Vec<Bar> = (0..num_bars)
            .map(|i| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1_000_000,
                idx: i,
            })
            .collect();

        // Only pass bars up to current_idx
        let visible_bars = &bars[..=current_idx];
        let atr = vec![1.0; visible_bars.len()];
        let adx = vec![25.0; visible_bars.len()];

        let state = MarketState::new(visible_bars, current_idx, &atr, &adx);

        // Verify no access to future
        prop_assert_eq!(state.bars.len(), current_idx + 1);
        prop_assert!(state.bars.last().unwrap().idx <= current_idx);
    }

    /// Position tracking updates correctly
    #[test]
    fn prop_position_tracking(
        entry_price in 50.0..150.0f64,
        bar_highs in prop::collection::vec(50.0..200.0f64, 1..20),
        bar_lows in prop::collection::vec(50.0..200.0f64, 1..20),
    ) {
        let signal = Signal::market(Direction::Long, 1.0, entry_price);
        let mut position = Position::new(
            0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            Direction::Long,
            10000.0,
            signal,
        );

        // Process some bars
        let len = bar_highs.len().min(bar_lows.len());
        for i in 0..len {
            let high = bar_highs[i];
            let low = bar_lows[i].min(high);
            let bar = Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0,
                high,
                low,
                close: 100.0,
                volume: 1_000_000,
                idx: i + 1,
            };
            position.update_for_bar(&bar);
        }

        // Verify tracking
        prop_assert!(position.high_since_entry >= entry_price);
        prop_assert!(position.low_since_entry <= entry_price);
        prop_assert_eq!(position.bars_held, len);
    }

    /// Direction opposite is involutive
    #[test]
    fn prop_direction_opposite_involutive(is_long: bool) {
        let dir = if is_long { Direction::Long } else { Direction::Short };
        prop_assert_eq!(dir.opposite().opposite(), dir);
    }

    /// Direction sign is correct
    #[test]
    fn prop_direction_sign(is_long: bool) {
        let dir = if is_long { Direction::Long } else { Direction::Short };
        if is_long {
            prop_assert_eq!(dir.sign(), 1.0);
        } else {
            prop_assert_eq!(dir.sign(), -1.0);
        }
    }

    /// Signal strength is clamped to [0, 1]
    #[test]
    fn prop_signal_strength_clamped(raw_strength in -2.0..3.0f64) {
        let signal = Signal::market(Direction::Long, raw_strength, 100.0);
        prop_assert!(signal.strength >= 0.0);
        prop_assert!(signal.strength <= 1.0);
    }

    /// ParamValue accessors return correct types
    #[test]
    fn prop_param_value_int(val in -1000i64..1000i64) {
        let pv = ParamValue::Int(val);
        prop_assert_eq!(pv.as_int(), Some(val));
        prop_assert!(pv.as_float().is_none());
        prop_assert!(pv.as_bool().is_none());
        prop_assert!(pv.as_choice().is_none());
    }

    /// ParamDef cardinality is correct for int ranges
    #[test]
    fn prop_param_cardinality_int(
        min in 0i64..50i64,
        range in 1i64..50i64,
        step in 1i64..10i64,
    ) {
        let max = min + range;
        let def = ParamDef::int("test", min, max, step);
        let expected = ((max - min) / step + 1) as usize;
        prop_assert_eq!(def.cardinality(), expected);
    }

    /// FillResult cost is sum of slippage and commission
    #[test]
    fn prop_fill_result_cost(
        price in 50.0..200.0f64,
        slippage in 0.0..1.0f64,
        commission in 0.0..5.0f64,
    ) {
        let fill = FillResult::filled(price, 0, slippage, commission);
        prop_assert!((fill.total_cost() - (slippage + commission)).abs() < 1e-10);
    }

    /// Trade return calculation is correct for long
    #[test]
    fn prop_trade_return_long(
        entry_price in 50.0..150.0f64,
        return_pct in -0.5..1.0f64,
    ) {
        let exit_price = entry_price * (1.0 + return_pct);
        let trade = Trade::new(
            0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            10,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            exit_price,
            Direction::Long,
            10000.0,
            ExitReason::StopHit,
            entry_price * 1.1,
            entry_price * 0.9,
            10,
        );

        prop_assert!((trade.return_pct - return_pct).abs() < 1e-10);
    }

    /// Trade is winner iff return > 0
    #[test]
    fn prop_trade_winner_loser(return_pct in -0.5..0.5f64) {
        let entry_price = 100.0;
        let exit_price = entry_price * (1.0 + return_pct);
        let trade = Trade::new(
            0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            10,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            exit_price,
            Direction::Long,
            10000.0,
            ExitReason::StopHit,
            entry_price * 1.1,
            entry_price * 0.9,
            10,
        );

        prop_assert_eq!(trade.is_winner(), return_pct > 0.0);
        prop_assert_eq!(trade.is_loser(), return_pct < 0.0);
    }
}

#[cfg(test)]
mod determinism_tests {
    use super::*;

    /// MarketState lookback returns correct number of bars
    #[test]
    fn test_lookback_length() {
        let bars: Vec<Bar> = (0..20)
            .map(|i| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 102.0,
                volume: 1_000_000,
                idx: i,
            })
            .collect();

        let atr = vec![1.0; 20];
        let adx = vec![25.0; 20];

        // At bar 10, lookback(5) should return bars 5-10 (6 bars)
        let state = MarketState::new(&bars[..11], 10, &atr[..11], &adx[..11]);
        let lb = state.lookback(5);
        assert_eq!(lb.len(), 6);
        assert_eq!(lb.first().unwrap().idx, 5);
        assert_eq!(lb.last().unwrap().idx, 10);
    }

    /// Highest high excludes current bar
    #[test]
    fn test_highest_high_excludes_current() {
        let mut bars: Vec<Bar> = (0..10)
            .map(|i| Bar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0,
                high: 100.0 + i as f64, // Increasing highs
                low: 95.0,
                close: 100.0,
                volume: 1_000_000,
                idx: i,
            })
            .collect();

        // Make current bar have the highest high
        bars[9].high = 200.0;

        let atr = vec![1.0; 10];
        let adx = vec![25.0; 10];
        let state = MarketState::new(&bars, 9, &atr, &adx);

        // Should return 108.0 (bar 8), not 200.0 (bar 9)
        let hh = state.highest_high(5);
        assert_eq!(hh, 108.0);
    }

    /// Position extremes start at entry price
    #[test]
    fn test_position_extremes_from_entry() {
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let position = Position::new(
            5,
            NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),
            100.0,
            Direction::Long,
            10000.0,
            signal,
        );

        // Critical invariant: extremes start at entry price
        assert_eq!(position.high_since_entry, 100.0);
        assert_eq!(position.low_since_entry, 100.0);
    }

    /// NoFilter always allows and never forces exit
    #[test]
    fn test_no_filter() {
        let filter = NoFilter;
        let signal = Signal::market(Direction::Long, 1.0, 100.0);
        let bar = Bar {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 102.0,
            volume: 1_000_000,
            idx: 0,
        };
        let atr = [1.0];
        let adx = [25.0];
        let bars = [bar.clone()];
        let state = MarketState::new(&bars, 0, &atr, &adx);

        assert!(filter.allow_signal(&signal, &bar, &state));

        let position = Position::new(
            0,
            bar.date,
            100.0,
            Direction::Long,
            10000.0,
            signal,
        );
        assert!(!filter.force_exit(&position, &bar, &state));
    }
}
