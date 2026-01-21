//! Execution model trait.
//!
//! ExecutionModels simulate how orders become fills with realistic assumptions
//! about timing, slippage, gaps, and fees.

use crate::param::ParamDef;
use crate::types::{Bar, FillResult, GapPolicy, Order, Position};

/// Execution model component.
///
/// # Contract
///
/// - MUST explicitly declare fill timing and gap policy
/// - MUST NOT peek at future bars
/// - SHOULD apply realistic slippage and commission
///
/// # Fill Timing
///
/// Different execution models make different assumptions:
/// - NextOpenFill: Order generated on bar N fills at bar N+1 open
/// - CloseFill: Order fills at current bar's close
/// - IntradayFill: More complex intrabar assumptions
///
/// # Example
/// ```ignore
/// struct NextOpenFill {
///     slippage_bps: f64,
///     commission_per_trade: f64,
/// }
///
/// impl ExecutionModel for NextOpenFill {
///     fn attempt_fill(
///         &self,
///         order: &Order,
///         signal_bar: &Bar,
///         fill_bar: &Bar,
///     ) -> FillResult {
///         let base_price = fill_bar.open;
///         let slippage = base_price * self.slippage_bps / 10000.0;
///
///         // Long orders get adverse slippage (higher price)
///         let fill_price = match order.direction {
///             Direction::Long => base_price + slippage,
///             Direction::Short => base_price - slippage,
///         };
///
///         FillResult::filled(
///             fill_price,
///             fill_bar.idx,
///             slippage,
///             self.commission_per_trade,
///         )
///     }
/// }
/// ```
pub trait ExecutionModel: Send + Sync {
    /// Unique identifier for logging.
    fn name(&self) -> &str;

    /// Attempt to fill an order.
    ///
    /// # Arguments
    /// - `order`: The order to fill
    /// - `signal_bar`: The bar when the signal was generated
    /// - `fill_bar`: The bar when fill is attempted (typically next bar)
    fn attempt_fill(&self, order: &Order, signal_bar: &Bar, fill_bar: &Bar) -> FillResult;

    /// Check if a stop was hit during a bar.
    ///
    /// # Returns
    /// `Some(fill_price)` if stop was hit, `None` otherwise
    fn check_stop(&self, position: &Position, bar: &Bar) -> Option<f64>;

    /// Gap policy for this execution model.
    fn gap_policy(&self) -> GapPolicy;

    /// Slippage in basis points (for reporting).
    fn slippage_bps(&self) -> f64;

    /// Commission per trade (for reporting).
    fn commission(&self) -> f64;

    /// Parameter specification for Monte Carlo sampling.
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn ExecutionModel>;
}

impl Clone for Box<dyn ExecutionModel> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[cfg(test)]
mod tests {
    // Trait tests would go here once we have implementations
}
