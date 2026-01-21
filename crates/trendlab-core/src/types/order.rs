//! Order types for execution.

use super::{Direction, Signal};
use serde::{Deserialize, Serialize};

/// Type of order for execution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    /// Execute at best available price
    Market,
    /// Execute at specified price or better
    Limit(f64),
    /// Execute when price reaches trigger
    Stop(f64),
    /// Stop with limit (stop triggers, then limit order)
    StopLimit {
        /// Stop trigger price
        stop: f64,
        /// Limit price after trigger
        limit: f64,
    },
}

impl OrderType {
    /// Returns the reference price for this order type, if any.
    pub fn reference_price(&self) -> Option<f64> {
        match self {
            OrderType::Market => None,
            OrderType::Limit(p) => Some(*p),
            OrderType::Stop(p) => Some(*p),
            OrderType::StopLimit { stop, .. } => Some(*stop),
        }
    }
}

/// Order generated from a Signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Trade direction
    pub direction: Direction,
    /// Order type
    pub order_type: OrderType,
    /// Position size in dollars
    pub size: f64,
    /// Signal that triggered this order
    pub signal: Signal,
}

impl Order {
    /// Create a market order from a signal.
    pub fn market(signal: Signal, size: f64) -> Self {
        Self {
            direction: signal.direction,
            order_type: OrderType::Market,
            size,
            signal,
        }
    }

    /// Create a limit order from a signal.
    pub fn limit(signal: Signal, limit_price: f64, size: f64) -> Self {
        Self {
            direction: signal.direction,
            order_type: OrderType::Limit(limit_price),
            size,
            signal,
        }
    }

    /// Create a stop order from a signal.
    pub fn stop(signal: Signal, stop_price: f64, size: f64) -> Self {
        Self {
            direction: signal.direction,
            order_type: OrderType::Stop(stop_price),
            size,
            signal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_type_reference_price() {
        assert_eq!(OrderType::Market.reference_price(), None);
        assert_eq!(OrderType::Limit(100.0).reference_price(), Some(100.0));
        assert_eq!(OrderType::Stop(95.0).reference_price(), Some(95.0));
        assert_eq!(
            OrderType::StopLimit {
                stop: 95.0,
                limit: 94.0
            }
            .reference_price(),
            Some(95.0)
        );
    }

    #[test]
    fn test_market_order() {
        let sig = Signal::market(Direction::Long, 1.0, 100.0);
        let order = Order::market(sig, 10000.0);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.size, 10000.0);
    }
}
