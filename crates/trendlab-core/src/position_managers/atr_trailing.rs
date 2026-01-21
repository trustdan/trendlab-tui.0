//! ATR Trailing Stop Position Manager.
//!
//! Tracks the highest price SINCE ENTRY and places a stop at
//! `high_since_entry - (atr_multiplier * ATR)`.
//!
//! Exit reference mode: SinceEntryTrailingExtreme

use crate::exit_reference::ExitReferenceMode;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};
use crate::traits::PositionManager;
use crate::types::{Action, Bar, Direction, ExitReason, Position, Signal};

/// ATR Trailing Stop Position Manager.
///
/// # Strategy
///
/// This position manager implements a volatility-adjusted trailing stop:
/// - Stop is placed at `high_since_entry - (multiplier * ATR)` for longs
/// - Stop is placed at `low_since_entry + (multiplier * ATR)` for shorts
/// - Stop only ratchets in the favorable direction (never moves against the trade)
///
/// # Exit Reference Mode
///
/// Uses `SinceEntryTrailingExtreme` - the reference (high/low) is tracked from
/// entry forward, NOT from historical data. This prevents the stickiness problem.
///
/// # Parameters
///
/// - `atr_multiplier`: Distance from extreme to stop in ATR units (default: 3.0)
///
/// # Example
///
/// ```ignore
/// let pm = AtrTrailingStop::new(2.5);
/// // Stop will trail 2.5 ATRs below the highest price since entry
/// ```
#[derive(Debug, Clone)]
pub struct AtrTrailingStop {
    atr_multiplier: f64,
    // Internal state (reset per trade)
    stop_price: Option<f64>,
    high_since_entry: f64,
    low_since_entry: f64,
}

impl AtrTrailingStop {
    /// Create a new ATR trailing stop position manager.
    ///
    /// # Arguments
    /// - `atr_multiplier`: Distance from extreme to stop in ATR units
    pub fn new(atr_multiplier: f64) -> Self {
        Self {
            atr_multiplier,
            stop_price: None,
            high_since_entry: 0.0,
            low_since_entry: f64::MAX,
        }
    }
}

impl Default for AtrTrailingStop {
    fn default() -> Self {
        Self::new(3.0)
    }
}

impl PositionManager for AtrTrailingStop {
    fn name(&self) -> &str {
        "ATRTrailingStop"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal) {
        // CRITICAL: Initialize from entry, NOT from historical data!
        self.high_since_entry = entry_price;
        self.low_since_entry = entry_price;

        // Use entry bar's range as initial ATR estimate
        let bar_range = entry_bar.high - entry_bar.low;
        let initial_atr = if bar_range > 0.0 { bar_range } else { 1.0 };

        // Set initial stop based on entry price and direction
        self.stop_price = Some(match signal.direction {
            Direction::Long => entry_price - (self.atr_multiplier * initial_atr),
            Direction::Short => entry_price + (self.atr_multiplier * initial_atr),
        });
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
        let atr = state.current_atr();
        if atr <= 0.0 {
            return Action::Hold;
        }

        // Update from engine-tracked position (authoritative source)
        self.high_since_entry = position.high_since_entry;
        self.low_since_entry = position.low_since_entry;

        // Calculate new trailing stop based on direction
        let new_stop = match position.direction {
            Direction::Long => self.high_since_entry - (self.atr_multiplier * atr),
            Direction::Short => self.low_since_entry + (self.atr_multiplier * atr),
        };

        // Only ratchet stop in favorable direction
        let should_update = match position.direction {
            Direction::Long => {
                self.stop_price.map(|s| new_stop > s).unwrap_or(true)
            }
            Direction::Short => {
                self.stop_price.map(|s| new_stop < s).unwrap_or(true)
            }
        };

        if should_update {
            self.stop_price = Some(new_stop);
            return Action::AdjustStop(new_stop);
        }

        // Check if stop would be hit on this bar
        // Note: The engine's ExecutionModel also checks this, but we can signal intent
        let stop = self.stop_price.unwrap_or(0.0);
        let stop_hit = match position.direction {
            Direction::Long => bar.low <= stop,
            Direction::Short => bar.high >= stop,
        };

        if stop_hit {
            Action::Exit(ExitReason::StopHit)
        } else {
            Action::Hold
        }
    }

    fn stop_price(&self) -> Option<f64> {
        self.stop_price
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![ParamDef {
            name: "atr_multiplier".into(),
            param_type: ParamType::Float {
                min: 1.0,
                max: 5.0,
                step: 0.5,
            },
            description: Some("ATR multiplier for stop distance".into()),
        }]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        // Return a fresh instance (no position-specific state)
        Box::new(AtrTrailingStop::new(self.atr_multiplier))
    }

    fn reset(&mut self) {
        self.stop_price = None;
        self.high_since_entry = 0.0;
        self.low_since_entry = f64::MAX;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bar(idx: usize, open: f64, high: f64, low: f64, close: f64) -> Bar {
        Bar {
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days(idx as i64),
            open,
            high,
            low,
            close,
            volume: 1_000_000,
            idx,
        }
    }

    fn make_position(entry_price: f64, direction: Direction) -> Position {
        Position::new(
            0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            direction,
            10000.0,
            Signal::market(direction, 1.0, entry_price),
        )
    }

    #[allow(dead_code)]
    fn make_state<'a>(bar: &'a Bar, atr: f64, adx: f64) -> MarketState<'a> {
        let bars = std::slice::from_ref(bar);
        let atr_slice: &'a [f64] = Box::leak(Box::new([atr]));
        let adx_slice: &'a [f64] = Box::leak(Box::new([adx]));
        MarketState::new(bars, 0, atr_slice, adx_slice)
    }

    #[test]
    fn test_on_entry_initializes_from_entry_price() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));

        // High/low since entry should start at entry price
        assert_eq!(pm.high_since_entry, 100.0);
        assert_eq!(pm.low_since_entry, 100.0);

        // Stop should be set
        assert!(pm.stop_price.is_some());
        // Bar range = 102 - 98 = 4, so stop = 100 - 2*4 = 92
        assert!((pm.stop_price.unwrap() - 92.0).abs() < 1e-10);
    }

    #[test]
    fn test_stop_ratchets_up_for_long() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));

        // Create position that moved favorably
        let mut position = make_position(100.0, Direction::Long);
        position.high_since_entry = 110.0; // Price went up

        let bar = make_bar(1, 108.0, 111.0, 107.0, 109.0);
        let atr = 4.0;

        // Create a proper state with leaked memory (for tests only)
        let bars: &'static [Bar] = Box::leak(Box::new([bar.clone()]));
        let atr_slice: &'static [f64] = Box::leak(Box::new([atr]));
        let adx_slice: &'static [f64] = Box::leak(Box::new([25.0]));
        let state = MarketState::new(bars, 0, atr_slice, adx_slice);

        let action = pm.on_bar(&bar, &position, &state);

        // Stop should have ratcheted up: 110 - 2*4 = 102
        match action {
            Action::AdjustStop(stop) => {
                assert!((stop - 102.0).abs() < 1e-10);
            }
            _ => panic!("Expected AdjustStop, got {:?}", action),
        }
    }

    #[test]
    fn test_stop_does_not_ratchet_down_for_long() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));

        // First, make the stop ratchet up
        let mut position = make_position(100.0, Direction::Long);
        position.high_since_entry = 110.0;

        let bar1 = make_bar(1, 108.0, 111.0, 107.0, 109.0);
        let bars1: &'static [Bar] = Box::leak(Box::new([bar1.clone()]));
        let atr_slice: &'static [f64] = Box::leak(Box::new([4.0]));
        let adx_slice: &'static [f64] = Box::leak(Box::new([25.0]));
        let state1 = MarketState::new(bars1, 0, atr_slice, adx_slice);

        pm.on_bar(&bar1, &position, &state1);

        let stop_after_up = pm.stop_price.unwrap();

        // Now simulate price dropping but high_since_entry stays at 110
        // (high_since_entry never goes down)
        let bar2 = make_bar(2, 105.0, 106.0, 104.0, 105.0);
        let bars2: &'static [Bar] = Box::leak(Box::new([bar2.clone()]));
        let atr_slice2: &'static [f64] = Box::leak(Box::new([4.0]));
        let adx_slice2: &'static [f64] = Box::leak(Box::new([25.0]));
        let state2 = MarketState::new(bars2, 0, atr_slice2, adx_slice2);

        let action = pm.on_bar(&bar2, &position, &state2);

        // Stop should NOT have moved down
        match action {
            Action::Hold => {
                assert_eq!(pm.stop_price.unwrap(), stop_after_up);
            }
            _ => panic!("Expected Hold, got {:?}", action),
        }
    }

    #[test]
    fn test_stop_hit_returns_exit() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));
        // Stop is at 92

        let mut position = make_position(100.0, Direction::Long);
        position.high_since_entry = 100.0;

        // Bar that hits the stop
        let bar = make_bar(1, 93.0, 94.0, 91.0, 92.0);
        let bars: &'static [Bar] = Box::leak(Box::new([bar.clone()]));
        let atr_slice: &'static [f64] = Box::leak(Box::new([4.0]));
        let adx_slice: &'static [f64] = Box::leak(Box::new([25.0]));
        let state = MarketState::new(bars, 0, atr_slice, adx_slice);

        let action = pm.on_bar(&bar, &position, &state);

        assert!(matches!(action, Action::Exit(ExitReason::StopHit)));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));

        assert!(pm.stop_price.is_some());
        assert_eq!(pm.high_since_entry, 100.0);

        pm.reset();

        assert!(pm.stop_price.is_none());
        assert_eq!(pm.high_since_entry, 0.0);
    }

    #[test]
    fn test_box_clone_returns_fresh_state() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 99.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Long, 1.0, 100.0));

        let cloned = pm.box_clone();

        // Cloned should have fresh state
        assert!(cloned.stop_price().is_none());
        assert_eq!(cloned.name(), "ATRTrailingStop");
    }

    #[test]
    fn test_short_position_trailing() {
        let mut pm = AtrTrailingStop::new(2.0);
        let entry_bar = make_bar(0, 101.0, 102.0, 98.0, 100.0);

        pm.on_entry(&entry_bar, 100.0, &Signal::market(Direction::Short, 1.0, 100.0));

        // For short, low_since_entry is tracked
        let mut position = make_position(100.0, Direction::Short);
        position.low_since_entry = 90.0; // Price went down (favorable for short)

        let bar = make_bar(1, 92.0, 93.0, 89.0, 91.0);
        let bars: &'static [Bar] = Box::leak(Box::new([bar.clone()]));
        let atr_slice: &'static [f64] = Box::leak(Box::new([4.0]));
        let adx_slice: &'static [f64] = Box::leak(Box::new([25.0]));
        let state = MarketState::new(bars, 0, atr_slice, adx_slice);

        let action = pm.on_bar(&bar, &position, &state);

        // Stop should be at: low_since_entry + multiplier * atr = 90 + 2*4 = 98
        match action {
            Action::AdjustStop(stop) => {
                assert!((stop - 98.0).abs() < 1e-10);
            }
            _ => panic!("Expected AdjustStop, got {:?}", action),
        }
    }

    #[test]
    fn test_parameter_spec() {
        let pm = AtrTrailingStop::new(2.5);
        let params = pm.parameter_spec();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "atr_multiplier");
    }

    #[test]
    fn test_exit_reference_mode() {
        let pm = AtrTrailingStop::new(2.0);
        assert_eq!(
            pm.exit_reference_mode(),
            Some(ExitReferenceMode::SinceEntryTrailingExtreme)
        );
    }
}
