//! Next Open Fill Execution Model.
//!
//! Fills orders at the next bar's open price with optional slippage and commission.

use crate::param::{ParamDef, ParamType};
use crate::traits::ExecutionModel;
use crate::types::{Bar, Direction, FillResult, GapPolicy, Order, Position};

/// Next Open Fill Execution Model.
///
/// # Behavior
///
/// - Entry orders fill at the next bar's open price
/// - Stop orders check if the bar breaches the stop level
/// - Gap-through stops fill at the open (more realistic)
///
/// # Parameters
///
/// - `slippage_bps`: Slippage in basis points (default: 5.0)
/// - `commission_per_trade`: Fixed commission per trade (default: 1.0)
///
/// # Example
///
/// ```ignore
/// let exec = NextOpenFill::new(5.0, 1.0);
/// // Orders will fill at next open + 0.05% slippage + $1 commission
/// ```
#[derive(Debug, Clone)]
pub struct NextOpenFill {
    slippage_bps: f64,
    commission_per_trade: f64,
}

impl NextOpenFill {
    /// Create a new next-open execution model.
    ///
    /// # Arguments
    /// - `slippage_bps`: Slippage in basis points
    /// - `commission_per_trade`: Fixed commission per trade
    pub fn new(slippage_bps: f64, commission_per_trade: f64) -> Self {
        Self {
            slippage_bps,
            commission_per_trade,
        }
    }

    /// Create a no-cost execution model (for testing).
    pub fn no_cost() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl Default for NextOpenFill {
    fn default() -> Self {
        Self::new(5.0, 1.0)
    }
}

impl ExecutionModel for NextOpenFill {
    fn name(&self) -> &str {
        "NextOpenFill"
    }

    fn attempt_fill(
        &self,
        order: &Order,
        _signal_bar: &Bar,
        fill_bar: &Bar,
    ) -> FillResult {
        let base_price = fill_bar.open;

        // Apply adverse slippage (higher for longs, lower for shorts)
        let slippage_amount = base_price * self.slippage_bps / 10000.0;
        let slippage = match order.direction {
            Direction::Long => slippage_amount,  // Pay more to buy
            Direction::Short => -slippage_amount, // Receive less to sell
        };

        let fill_price = base_price + slippage;

        FillResult::filled(
            fill_price,
            fill_bar.idx,
            slippage.abs(),
            self.commission_per_trade,
        )
    }

    fn check_stop(
        &self,
        position: &Position,
        bar: &Bar,
    ) -> Option<f64> {
        let stop = position.stop_price?;

        match position.direction {
            Direction::Long => {
                // Long position: stop is hit when price falls to stop level
                if bar.low <= stop {
                    if bar.open <= stop {
                        // Gap through - fill at open (worse than stop)
                        Some(bar.open)
                    } else {
                        // Normal stop hit - fill at stop price
                        Some(stop)
                    }
                } else {
                    None
                }
            }
            Direction::Short => {
                // Short position: stop is hit when price rises to stop level
                if bar.high >= stop {
                    if bar.open >= stop {
                        // Gap through - fill at open (worse than stop)
                        Some(bar.open)
                    } else {
                        // Normal stop hit - fill at stop price
                        Some(stop)
                    }
                } else {
                    None
                }
            }
        }
    }

    fn gap_policy(&self) -> GapPolicy {
        GapPolicy::FillAtOpen
    }

    fn slippage_bps(&self) -> f64 {
        self.slippage_bps
    }

    fn commission(&self) -> f64 {
        self.commission_per_trade
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "slippage_bps".into(),
                param_type: ParamType::Float {
                    min: 0.0,
                    max: 50.0,
                    step: 5.0,
                },
                description: Some("Slippage in basis points".into()),
            },
            ParamDef {
                name: "commission_per_trade".into(),
                param_type: ParamType::Float {
                    min: 0.0,
                    max: 10.0,
                    step: 1.0,
                },
                description: Some("Commission per trade in dollars".into()),
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn ExecutionModel> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Signal;
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

    fn make_order(direction: Direction, price: f64) -> Order {
        Order::market(Signal::market(direction, 1.0, price), 10000.0)
    }

    fn make_position(entry_price: f64, direction: Direction, stop: f64) -> Position {
        let mut pos = Position::new(
            0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            direction,
            10000.0,
            Signal::market(direction, 1.0, entry_price),
        );
        pos.set_stop(stop);
        pos
    }

    #[test]
    fn test_fill_at_open_long() {
        let exec = NextOpenFill::no_cost();
        let signal_bar = make_bar(0, 100.0, 102.0, 99.0, 101.0);
        let fill_bar = make_bar(1, 101.5, 103.0, 100.0, 102.0);
        let order = make_order(Direction::Long, 100.0);

        let result = exec.attempt_fill(&order, &signal_bar, &fill_bar);

        assert!(result.filled);
        assert_eq!(result.fill_price, 101.5); // Opens at 101.5
        assert_eq!(result.fill_bar_idx, 1);
    }

    #[test]
    fn test_fill_with_slippage_long() {
        let exec = NextOpenFill::new(10.0, 0.0); // 10 bps slippage
        let signal_bar = make_bar(0, 100.0, 102.0, 99.0, 101.0);
        let fill_bar = make_bar(1, 100.0, 103.0, 99.0, 102.0);
        let order = make_order(Direction::Long, 100.0);

        let result = exec.attempt_fill(&order, &signal_bar, &fill_bar);

        // 100.0 + 0.1% = 100.10
        assert!((result.fill_price - 100.10).abs() < 1e-10);
        assert!((result.slippage - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_fill_with_slippage_short() {
        let exec = NextOpenFill::new(10.0, 0.0); // 10 bps slippage
        let signal_bar = make_bar(0, 100.0, 102.0, 99.0, 101.0);
        let fill_bar = make_bar(1, 100.0, 103.0, 99.0, 102.0);
        let order = make_order(Direction::Short, 100.0);

        let result = exec.attempt_fill(&order, &signal_bar, &fill_bar);

        // 100.0 - 0.1% = 99.90 (worse for short)
        assert!((result.fill_price - 99.90).abs() < 1e-10);
    }

    #[test]
    fn test_fill_with_commission() {
        let exec = NextOpenFill::new(0.0, 5.0);
        let signal_bar = make_bar(0, 100.0, 102.0, 99.0, 101.0);
        let fill_bar = make_bar(1, 100.0, 103.0, 99.0, 102.0);
        let order = make_order(Direction::Long, 100.0);

        let result = exec.attempt_fill(&order, &signal_bar, &fill_bar);

        assert_eq!(result.commission, 5.0);
        assert_eq!(result.total_cost(), 5.0);
    }

    #[test]
    fn test_stop_hit_long() {
        let exec = NextOpenFill::no_cost();
        let position = make_position(100.0, Direction::Long, 95.0);
        let bar = make_bar(1, 98.0, 99.0, 94.0, 96.0);

        let fill = exec.check_stop(&position, &bar);

        // Stop at 95, bar low is 94, so stop is hit at 95
        assert!(fill.is_some());
        assert_eq!(fill.unwrap(), 95.0);
    }

    #[test]
    fn test_stop_gap_through_long() {
        let exec = NextOpenFill::no_cost();
        let position = make_position(100.0, Direction::Long, 95.0);
        let bar = make_bar(1, 93.0, 94.0, 92.0, 93.5);

        let fill = exec.check_stop(&position, &bar);

        // Stop at 95, but bar opens at 93 (gap through) - fill at open
        assert!(fill.is_some());
        assert_eq!(fill.unwrap(), 93.0);
    }

    #[test]
    fn test_stop_not_hit_long() {
        let exec = NextOpenFill::no_cost();
        let position = make_position(100.0, Direction::Long, 95.0);
        let bar = make_bar(1, 99.0, 102.0, 96.0, 101.0);

        let fill = exec.check_stop(&position, &bar);

        // Stop at 95, bar low is 96 - not hit
        assert!(fill.is_none());
    }

    #[test]
    fn test_stop_hit_short() {
        let exec = NextOpenFill::no_cost();
        let position = make_position(100.0, Direction::Short, 105.0);
        let bar = make_bar(1, 103.0, 106.0, 102.0, 104.0);

        let fill = exec.check_stop(&position, &bar);

        // Stop at 105, bar high is 106 - hit at 105
        assert!(fill.is_some());
        assert_eq!(fill.unwrap(), 105.0);
    }

    #[test]
    fn test_stop_gap_through_short() {
        let exec = NextOpenFill::no_cost();
        let position = make_position(100.0, Direction::Short, 105.0);
        let bar = make_bar(1, 107.0, 108.0, 106.0, 107.5);

        let fill = exec.check_stop(&position, &bar);

        // Stop at 105, but bar opens at 107 (gap through) - fill at open
        assert!(fill.is_some());
        assert_eq!(fill.unwrap(), 107.0);
    }

    #[test]
    fn test_no_stop_set() {
        let exec = NextOpenFill::no_cost();
        let mut position = make_position(100.0, Direction::Long, 95.0);
        position.stop_price = None; // Clear the stop

        let bar = make_bar(1, 90.0, 91.0, 89.0, 90.5);
        let fill = exec.check_stop(&position, &bar);

        assert!(fill.is_none());
    }

    #[test]
    fn test_parameter_spec() {
        let exec = NextOpenFill::new(10.0, 2.0);
        let params = exec.parameter_spec();

        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "slippage_bps");
        assert_eq!(params[1].name, "commission_per_trade");
    }

    #[test]
    fn test_gap_policy() {
        let exec = NextOpenFill::default();
        assert_eq!(exec.gap_policy(), GapPolicy::FillAtOpen);
    }

    #[test]
    fn test_box_clone() {
        let exec = NextOpenFill::new(15.0, 3.0);
        let cloned = exec.box_clone();

        assert_eq!(cloned.name(), "NextOpenFill");
        assert_eq!(cloned.slippage_bps(), 15.0);
        assert_eq!(cloned.commission(), 3.0);
    }
}
