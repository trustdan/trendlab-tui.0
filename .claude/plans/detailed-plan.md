# TrendLab v2: Detailed Implementation Plan

> The turnkey guide for building a research-grade trend-following backtester in Rust.

---

## Table of Contents

1. [Phase 1: Foundation (trendlab-core)](#phase-1-foundation)
2. [Phase 2: Data Infrastructure (trendlab-data)](#phase-2-data-infrastructure)
3. [Phase 3: Engine Spine (trendlab-core)](#phase-3-engine-spine)
4. [Phase 4: YOLO Discovery (trendlab-yolo)](#phase-4-yolo-discovery)
5. [Phase 5: Terminal UI (trendlab-tui)](#phase-5-terminal-ui)
6. [Phase 6: Export & Pine Parity (trendlab-export)](#phase-6-export--pine-parity)

---

## Phase 1: Foundation

### 1.1 File Structure

```text
crates/trendlab-core/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Re-exports, module declarations
│   ├── types/
│   │   ├── mod.rs                # Module exports
│   │   ├── bar.rs                # Bar type (OHLCV)
│   │   ├── signal.rs             # Signal, SignalIntent, Direction
│   │   ├── order.rs              # Order, OrderType
│   │   ├── fill.rs               # Fill, FillResult, GapPolicy
│   │   ├── position.rs           # Position (open trade state)
│   │   ├── trade.rs              # Trade (closed), ExitReason
│   │   ├── action.rs             # PositionManager output
│   │   └── metrics.rs            # Metrics struct + calculations
│   ├── traits/
│   │   ├── mod.rs                # Trait exports
│   │   ├── signal_generator.rs   # SignalGenerator trait
│   │   ├── position_manager.rs   # PositionManager trait
│   │   ├── execution_model.rs    # ExecutionModel trait
│   │   └── signal_filter.rs      # SignalFilter trait
│   ├── exit_reference.rs         # ExitReferenceMode enum
│   ├── market_state.rs           # Read-only market context
│   ├── param.rs                  # ParamDef, ParamValue for Monte Carlo
│   └── error.rs                  # Error types
├── tests/
│   ├── property_tests.rs         # proptest-based invariant tests
│   └── determinism_tests.rs      # Reproducibility tests
```

### 1.2 Core Types

#### Bar (OHLCV Data)

```rust
// crates/trendlab-core/src/types/bar.rs

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A single OHLCV bar representing one trading day.
///
/// # Invariants
/// - `high >= low`
/// - `high >= open` and `high >= close`
/// - `low <= open` and `low <= close`
/// - `volume >= 0`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    /// Trading date (no time component for daily bars)
    pub date: NaiveDate,

    /// Opening price
    pub open: f64,

    /// Highest price during the period
    pub high: f64,

    /// Lowest price during the period
    pub low: f64,

    /// Closing price
    pub close: f64,

    /// Trading volume
    pub volume: u64,

    /// Zero-based index in the bar sequence
    pub idx: usize,
}

impl Bar {
    /// True range for this bar (requires previous close for accuracy)
    pub fn true_range(&self, prev_close: Option<f64>) -> f64 {
        let hl = self.high - self.low;
        match prev_close {
            Some(pc) => {
                let hc = (self.high - pc).abs();
                let lc = (self.low - pc).abs();
                hl.max(hc).max(lc)
            }
            None => hl,
        }
    }

    /// Typical price: (high + low + close) / 3
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }
}
```

#### Direction

```rust
// crates/trendlab-core/src/types/direction.rs

use serde::{Deserialize, Serialize};

/// Trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Long,
    Short,
}

impl Direction {
    /// Returns the opposite direction.
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Long => Direction::Short,
            Direction::Short => Direction::Long,
        }
    }

    /// Returns +1.0 for Long, -1.0 for Short.
    pub fn sign(&self) -> f64 {
        match self {
            Direction::Long => 1.0,
            Direction::Short => -1.0,
        }
    }
}
```

#### Signal

```rust
// crates/trendlab-core/src/types/signal.rs

use super::Direction;
use serde::{Deserialize, Serialize};

/// Entry signal produced by a SignalGenerator.
///
/// # Contract
/// - Signals represent entry *intent*, not orders
/// - Signals MUST NOT contain exit information
/// - Signal strength should be normalized [0.0, 1.0] where practical
///   (generators should clamp if their raw strength can exceed 1.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Direction of the proposed trade
    pub direction: Direction,

    /// Optional limit/stop price for entry (None = market order)
    pub entry_level: Option<f64>,

    /// Signal strength or confidence [0.0, 1.0]
    pub strength: f64,

    /// The indicator value that triggered the signal (for debugging/export)
    pub trigger_value: f64,
}
```

#### Order and OrderType

```rust
// crates/trendlab-core/src/types/order.rs

use super::{Direction, Signal};
use serde::{Deserialize, Serialize};

/// Order generated from a Signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub direction: Direction,
    pub order_type: OrderType,
    pub size: f64,
    pub signal: Signal,
}

/// Type of order for execution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    /// Execute at best available price
    Market,
    /// Execute at specified price or better
    Limit(f64),
    /// Execute when price reaches trigger
    Stop(f64),
    /// Stop with limit
    StopLimit { stop: f64, limit: f64 },
}
```

#### Fill and FillResult

```rust
// crates/trendlab-core/src/types/fill.rs

use serde::{Deserialize, Serialize};

/// Result of an order execution attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillResult {
    /// Whether the order was filled
    pub filled: bool,

    /// Price at which the order was filled (if filled)
    pub fill_price: f64,

    /// Bar index when fill occurred
    pub fill_bar_idx: usize,

    /// Slippage incurred (signed: positive = adverse)
    pub slippage: f64,

    /// Commission charged
    pub commission: f64,
}

/// Policy for handling gaps through stop prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapPolicy {
    /// Fill at open price if gap through stop
    FillAtOpen,
    /// Fill at stop price (unrealistic but sometimes used)
    FillAtStop,
    /// No fill on gap (position remains open)
    NoFill,
}
```

#### Exit Reference Mode (Critical for Stickiness Prevention)

```rust
// crates/trendlab-core/src/exit_reference.rs

use serde::{Deserialize, Serialize};

/// Exit reference mode for extreme-based exits.
///
/// # The Stickiness Problem
///
/// In v1, strategies using rolling references (e.g., 52-week high) for BOTH
/// entry AND exit created "sticky" positions that couldn't exit because the
/// reference kept moving away.
///
/// # The Solution
///
/// Every PositionManager that uses price extremes for exit calculations
/// MUST declare its exit reference mode. This makes the behavior explicit
/// and allows the system to detect potentially problematic configurations.
///
/// # Modes
///
/// - `EntryFrozenReference`: Reference is fixed at entry and never updates.
///   Example: "Stop at 10% below the high on the day I entered"
///
/// - `SinceEntryTrailingExtreme`: Reference tracks the extreme since entry.
///   Example: "Stop at 10% below the highest price since I entered"
///   Note: This is different from global rolling high!
///
/// - `SeparateEntryExitLookbacks`: Entry and exit use different windows.
///   Example: "Enter on 200-day high breakout, exit on 50-day low breakdown"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReferenceMode {
    /// Reference fixed at entry, never updates
    EntryFrozenReference,

    /// Tracks extreme since entry (NOT globally)
    SinceEntryTrailingExtreme,

    /// Separate lookback windows for entry vs exit
    SeparateEntryExitLookbacks,
}
```

#### Position (Open Trade State)

```rust
// crates/trendlab-core/src/types/position.rs

use super::{Direction, Signal};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Open position during backtest.
///
/// # Key Invariant
///
/// `high_since_entry` and `low_since_entry` are tracked FROM ENTRY FORWARD,
/// not from historical data. This prevents the stickiness problem where
/// exit references "run away" with global extremes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Bar index when position was opened
    pub entry_bar_idx: usize,

    /// Date of entry
    pub entry_date: NaiveDate,

    /// Fill price at entry
    pub entry_price: f64,

    /// Trade direction
    pub direction: Direction,

    /// Position size in dollars
    pub size: f64,

    /// Signal that triggered entry
    pub signal: Signal,

    /// Highest price SINCE ENTRY (not global!)
    pub high_since_entry: f64,

    /// Lowest price SINCE ENTRY (not global!)
    pub low_since_entry: f64,

    /// Number of bars held
    pub bars_held: usize,

    /// Current stop price (if any)
    pub stop_price: Option<f64>,
}

impl Position {
    /// Update tracking for a new bar
    pub fn update_for_bar(&mut self, bar: &super::Bar) {
        self.high_since_entry = self.high_since_entry.max(bar.high);
        self.low_since_entry = self.low_since_entry.min(bar.low);
        self.bars_held += 1;
    }

    /// Calculate unrealized P&L percentage
    pub fn unrealized_pnl_pct(&self, current_price: f64) -> f64 {
        match self.direction {
            Direction::Long => (current_price - self.entry_price) / self.entry_price,
            Direction::Short => (self.entry_price - current_price) / self.entry_price,
        }
    }
}
```

#### Action (PositionManager Output)

```rust
// crates/trendlab-core/src/types/action.rs

use serde::{Deserialize, Serialize};

/// Action returned by PositionManager on each bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Continue holding the position
    Hold,

    /// Adjust the stop price
    AdjustStop(f64),

    /// Scale out of the position by percentage
    ScaleOut { percent: f64, reason: ExitReason },

    /// Exit the entire position
    Exit(ExitReason),
}

/// Reason for exiting a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    /// Stop loss was hit
    StopHit,

    /// Take profit target reached
    TakeProfit,

    /// Signal-based exit
    SignalExit,

    /// Filter forced exit (regime change)
    FilterForceExit,

    /// Time-based exit (held too long)
    TimeExit,

    /// End of data reached
    EndOfData,
}
```

#### Trade (Completed Trade)

```rust
// crates/trendlab-core/src/types/trade.rs

use super::{Direction, ExitReason};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Completed trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub entry_bar_idx: usize,
    pub entry_date: NaiveDate,
    pub entry_price: f64,

    pub exit_bar_idx: usize,
    pub exit_date: NaiveDate,
    pub exit_price: f64,

    pub direction: Direction,
    pub size: f64,
    pub exit_reason: ExitReason,

    /// Return as decimal (0.05 = 5%)
    pub return_pct: f64,

    /// Bars held
    pub bars_held: usize,

    /// Maximum Adverse Excursion (worst drawdown during trade)
    pub mae: f64,

    /// Maximum Favorable Excursion (best gain during trade)
    pub mfe: f64,
}
```

#### Metrics

```rust
// crates/trendlab-core/src/types/metrics.rs

use serde::{Deserialize, Serialize};

/// Aggregate performance metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    /// Total return as decimal
    pub total_return: f64,

    /// Compound Annual Growth Rate
    pub cagr: f64,

    /// Sharpe ratio (annualized)
    pub sharpe: f64,

    /// Sortino ratio (annualized)
    pub sortino: f64,

    /// Maximum drawdown as decimal
    pub max_drawdown: f64,

    /// Win rate as decimal
    pub win_rate: f64,

    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,

    /// Total number of trades
    pub total_trades: usize,

    /// Average bars held per trade
    pub avg_bars_held: f64,

    /// Average trade return
    pub avg_return: f64,

    /// Average winner return
    pub avg_winner: f64,

    /// Average loser return
    pub avg_loser: f64,
}
```

#### MarketState (Read-Only Context)

```rust
// crates/trendlab-core/src/market_state.rs

use crate::types::Bar;

/// Read-only market context available to components.
///
/// # Lookahead Prevention
///
/// This struct provides a controlled view of market data. Components receive
/// only bars up to and including the current bar index, preventing lookahead bias.
///
/// # Contract
/// - `bars.len() == current_idx + 1`
/// - All indicator arrays have same length as bars
#[derive(Debug, Clone)]
pub struct MarketState<'a> {
    /// Bars from index 0 to current_idx (inclusive)
    pub bars: &'a [Bar],

    /// Current bar index
    pub current_idx: usize,

    /// Pre-computed ATR (14-period) at each bar
    pub atr: &'a [f64],

    /// Pre-computed ADX (14-period) at each bar
    pub adx: &'a [f64],
}

impl<'a> MarketState<'a> {
    /// Get the current bar
    pub fn current_bar(&self) -> &Bar {
        &self.bars[self.current_idx]
    }

    /// Get bars from N periods ago to current
    pub fn lookback(&self, periods: usize) -> &[Bar] {
        let start = self.current_idx.saturating_sub(periods);
        &self.bars[start..=self.current_idx]
    }

    /// Highest high over last N bars (excluding current)
    pub fn highest_high(&self, periods: usize) -> f64 {
        self.bars.iter()
            .rev()
            .skip(1) // Skip current bar
            .take(periods)
            .map(|b| b.high)
            .fold(f64::MIN, f64::max)
    }

    /// Lowest low over last N bars (excluding current)
    pub fn lowest_low(&self, periods: usize) -> f64 {
        self.bars.iter()
            .rev()
            .skip(1) // Skip current bar
            .take(periods)
            .map(|b| b.low)
            .fold(f64::MAX, f64::min)
    }

    /// Current ATR value
    pub fn current_atr(&self) -> f64 {
        self.atr.get(self.current_idx).copied().unwrap_or(0.0)
    }

    /// Current ADX value
    pub fn current_adx(&self) -> f64 {
        self.adx.get(self.current_idx).copied().unwrap_or(0.0)
    }
}
```

#### Parameter Definitions (for Monte Carlo)

```rust
// crates/trendlab-core/src/param.rs

use serde::{Deserialize, Serialize};

/// Parameter definition for Monte Carlo sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub param_type: ParamType,
}

/// Parameter type with sampling bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    Int { min: i64, max: i64, step: i64 },
    Float { min: f64, max: f64, step: f64 },
    Bool,
    Choice(Vec<String>),
}

/// Concrete parameter value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Choice(String),
}
```

### 1.3 Component Traits

#### SignalGenerator

```rust
// crates/trendlab-core/src/traits/signal_generator.rs

use crate::types::{Bar, Signal};
use crate::market_state::MarketState;
use crate::param::ParamDef;

/// Signal generator component.
///
/// # Contract
///
/// - MUST produce entry signals only (no exit logic)
/// - MUST NOT track position state or know about current holdings
/// - MUST NOT peek beyond bar N when processing bar N
/// - SHOULD return None during warmup period
///
/// # State
///
/// SignalGenerators may maintain indicator state (e.g., moving average values)
/// but MUST NOT maintain position state. State is per-indicator, not per-trade.
pub trait SignalGenerator: Send + Sync {
    /// Unique identifier for logging and leaderboards
    fn name(&self) -> &str;

    /// Minimum bars required before signals are valid
    fn warmup_bars(&self) -> usize;

    /// Generate an entry signal for the current bar.
    ///
    /// # Arguments
    /// - `bar`: The current bar being processed
    /// - `state`: Market state up to and including current bar
    ///
    /// # Returns
    /// - `Some(Signal)` if entry conditions are met
    /// - `None` if no entry signal
    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal>;

    /// Parameter specification for Monte Carlo sampling
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object
    fn box_clone(&self) -> Box<dyn SignalGenerator>;
}
```

#### PositionManager

```rust
// crates/trendlab-core/src/traits/position_manager.rs

use crate::types::{Bar, Position, Signal, Action};
use crate::market_state::MarketState;
use crate::exit_reference::ExitReferenceMode;
use crate::param::ParamDef;

/// Position manager component.
///
/// # Contract
///
/// - MUST initialize state at entry (via `on_entry`)
/// - MUST declare exit reference mode for extreme-based exits
/// - MUST NOT access SignalGenerator's internal state
/// - MUST output actions only (Hold, AdjustStop, Exit), not fills
///
/// # State Initialization
///
/// Critical: State like `high_since_entry` starts from the ENTRY bar,
/// not from historical data. This prevents the stickiness problem.
///
/// # Example
/// ```ignore
/// fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, _signal: &Signal) {
///     // Initialize from entry, NOT from historical high!
///     self.high_since_entry = entry_price;
///     self.stop_price = entry_price - (self.atr_multiplier * entry_bar.atr);
/// }
/// ```
pub trait PositionManager: Send + Sync {
    /// Unique identifier for logging and leaderboards
    fn name(&self) -> &str;

    /// Exit reference mode (required for extreme-based exits).
    ///
    /// Return `None` only if this PM doesn't use price extremes for exits.
    fn exit_reference_mode(&self) -> Option<ExitReferenceMode>;

    /// Initialize state when a new position is opened.
    ///
    /// # Important
    /// This is called AFTER the fill occurs. State should be
    /// initialized from the entry context, not historical data.
    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, signal: &Signal);

    /// Process a bar and return the management action.
    ///
    /// # Arguments
    /// - `bar`: Current bar
    /// - `position`: Current position state (maintained by engine)
    /// - `state`: Market state
    ///
    /// # Returns
    /// Action to take: Hold, AdjustStop, or Exit
    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action;

    /// Current stop price (for logging and execution model)
    fn stop_price(&self) -> Option<f64>;

    /// Parameter specification for Monte Carlo sampling
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into a boxed trait object with FRESH state
    fn box_clone(&self) -> Box<dyn PositionManager>;
}
```

#### ExecutionModel

```rust
// crates/trendlab-core/src/traits/execution_model.rs

use crate::types::{Bar, Order, Position, FillResult, GapPolicy};
use crate::param::ParamDef;

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
pub trait ExecutionModel: Send + Sync {
    /// Unique identifier
    fn name(&self) -> &str;

    /// Attempt to fill an order.
    ///
    /// # Arguments
    /// - `order`: The order to fill
    /// - `signal_bar`: The bar when the signal was generated
    /// - `fill_bar`: The bar when fill is attempted (typically next bar)
    fn attempt_fill(
        &self,
        order: &Order,
        signal_bar: &Bar,
        fill_bar: &Bar,
    ) -> FillResult;

    /// Check if a stop was hit during a bar.
    ///
    /// # Returns
    /// `Some(fill_price)` if stop was hit, `None` otherwise
    fn check_stop(
        &self,
        position: &Position,
        bar: &Bar,
    ) -> Option<f64>;

    /// Gap policy for this execution model
    fn gap_policy(&self) -> GapPolicy;

    /// Parameter specification
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into boxed trait object
    fn box_clone(&self) -> Box<dyn ExecutionModel>;
}
```

#### SignalFilter

```rust
// crates/trendlab-core/src/traits/signal_filter.rs

use crate::types::{Bar, Signal, Position};
use crate::market_state::MarketState;
use crate::param::ParamDef;

/// Signal filter component.
///
/// # Contract
///
/// - MUST return boolean decisions only
/// - MUST NOT modify signals or positions
/// - MAY force exit of existing positions (regime change)
///
/// # Use Cases
///
/// - ADX trend filter: Only allow signals when ADX > threshold
/// - Volatility filter: Suppress signals in high/low volatility
/// - Seasonal filter: Avoid certain months
/// - Correlation filter: Reduce signals when correlation high
pub trait SignalFilter: Send + Sync {
    /// Unique identifier
    fn name(&self) -> &str;

    /// Whether to allow this entry signal.
    ///
    /// # Returns
    /// - `true`: Signal passes, can proceed to execution
    /// - `false`: Signal suppressed, no entry
    fn allow_signal(&self, signal: &Signal, bar: &Bar, state: &MarketState) -> bool;

    /// Whether to force exit of existing position.
    ///
    /// # Use Case
    /// Regime change (e.g., ADX drops below threshold during trade)
    fn force_exit(&self, position: &Position, bar: &Bar, state: &MarketState) -> bool;

    /// Parameter specification
    fn parameter_spec(&self) -> Vec<ParamDef>;

    /// Clone into boxed trait object
    fn box_clone(&self) -> Box<dyn SignalFilter>;
}
```

### 1.4 BDD Scenarios (Phase 1)

```gherkin
# features/core/component_isolation.feature

Feature: Component State Isolation
  As a researcher
  I want components to be independent
  So that fair comparison is possible

  Scenario: SignalGenerator does not access PositionManager state
    Given a SignalGenerator "DonchianBreakout"
    And a PositionManager "ATRTrailing" with internal high_since_entry = $150
    When SignalGenerator.generate() is called
    Then SignalGenerator has no access to high_since_entry
    And SignalGenerator only sees MarketState

  Scenario: PositionManager does not access SignalGenerator internals
    Given a PositionManager "ATRTrailing"
    And a SignalGenerator "DonchianBreakout" with internal lookback_high = $200
    When PositionManager.on_bar() is called
    Then PositionManager has no access to lookback_high
    And PositionManager only sees Position and MarketState

  Scenario: Components are fresh per backtest run
    Given a PositionManager used in backtest run 1
    And the PM had high_since_entry = $150 at end of run 1
    When a new backtest run 2 starts with same PM type
    Then PM state is freshly initialized
    And high_since_entry from run 1 does NOT leak into run 2
```

```gherkin
# features/core/exit_reference.feature

Feature: Exit Reference Semantics
  As a researcher
  I want explicit exit reference modes
  So that stickiness bugs are prevented

  Scenario: PositionManager must declare exit reference mode
    Given a PositionManager using price extremes for exit
    When exit_reference_mode() is called
    Then it returns a valid ExitReferenceMode
    And the mode is one of: EntryFrozenReference, SinceEntryTrailingExtreme, SeparateEntryExitLookbacks

  Scenario: SinceEntryTrailingExtreme tracks from entry
    Given a PositionManager with SinceEntryTrailingExtreme mode
    And entry at bar 100 with price $100
    And bar 101 has high $105
    And bar 102 has high $110
    When checking position state at bar 102
    Then high_since_entry is $110 (tracked from entry)
    And it is NOT the global 52-week high

  Scenario: EntryFrozenReference never updates
    Given a PositionManager with EntryFrozenReference mode
    And entry at bar 100 with price $100
    And bar 105 has high $120
    When checking reference at bar 105
    Then reference is still $100 (frozen at entry)
```

```gherkin
# features/core/no_lookahead.feature

Feature: No Lookahead Bias
  As a researcher
  I want to ensure no future data leaks into decisions
  So that backtests are realistic

  Scenario: MarketState contains only past data
    Given bars with indices [0, 1, 2, 3, 4, 5]
    When processing bar index 3
    Then MarketState.bars has length 4
    And MarketState.bars contains indices [0, 1, 2, 3]
    And bar index 4 is NOT accessible

  Scenario: SignalGenerator cannot access future bars
    Given a SignalGenerator processing bar N
    Then state.bars[N+1] would panic (out of bounds)
    And highest_high() only considers bars 0..N

  Scenario: ExecutionModel respects fill timing
    Given an order generated on bar N
    And NextOpenFill execution model
    When attempting to fill
    Then fill uses bar N+1 open price
    And bar N+1 must exist (otherwise no fill)
```

### 1.5 Property Tests

```rust
// crates/trendlab-core/tests/property_tests.rs

use proptest::prelude::*;
use trendlab_core::types::*;
use trendlab_core::market_state::MarketState;

proptest! {
    /// Bar high is always >= low
    #[test]
    fn prop_bar_high_gte_low(
        open in 1.0..1000.0f64,
        close in 1.0..1000.0f64,
    ) {
        let high = open.max(close) * (1.0 + 0.05);
        let low = open.min(close) * (1.0 - 0.05);

        let bar = Bar {
            date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            open,
            high,
            low,
            close,
            volume: 1_000_000,
            idx: 0,
        };

        prop_assert!(bar.high >= bar.low);
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
                date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
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

        let state = MarketState {
            bars: visible_bars,
            current_idx,
            atr: &atr,
            adx: &adx,
        };

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
        let mut position = Position {
            entry_bar_idx: 0,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            direction: Direction::Long,
            size: 10000.0,
            signal: Signal {
                direction: Direction::Long,
                entry_level: None,
                strength: 1.0,
                trigger_value: entry_price,
            },
            high_since_entry: entry_price,
            low_since_entry: entry_price,
            bars_held: 0,
            stop_price: None,
        };

        // Process some bars
        let len = bar_highs.len().min(bar_lows.len());
        for i in 0..len {
            let bar = Bar {
                date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                open: 100.0,
                high: bar_highs[i],
                low: bar_lows[i].min(bar_highs[i]),
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
}
```

### 1.6 Implementation Order (Phase 1)

1. **Error types** (`error.rs`) - Foundational
2. **Direction enum** - No dependencies
3. **Bar type** - No dependencies
4. **Signal type** - Depends on Direction
5. **Order types** - Depends on Direction, Signal
6. **Fill types** - No dependencies
7. **ExitReason and Action** - No dependencies
8. **Position type** - Depends on Direction, Signal
9. **Trade type** - Depends on Direction, ExitReason
10. **Metrics type** - No dependencies
11. **ExitReferenceMode** - No dependencies
12. **ParamDef/ParamValue** - No dependencies
13. **MarketState** - Depends on Bar
14. **SignalGenerator trait** - Depends on Bar, Signal, MarketState, ParamDef
15. **PositionManager trait** - Depends on Bar, Position, Signal, Action, ExitReferenceMode, MarketState, ParamDef
16. **ExecutionModel trait** - Depends on Bar, Order, Position, FillResult, GapPolicy, ParamDef
17. **SignalFilter trait** - Depends on Bar, Signal, Position, MarketState, ParamDef
18. **Property tests** - After all types defined
19. **BDD step definitions** - After traits defined

---

## Phase 2: Data Infrastructure

### 2.1 File Structure

```text
crates/trendlab-data/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Re-exports
│   ├── client/
│   │   ├── mod.rs
│   │   └── yahoo.rs              # Yahoo Finance client
│   ├── cache/
│   │   ├── mod.rs
│   │   └── parquet.rs            # Parquet caching
│   ├── universe.rs               # Symbol universe management
│   ├── indicators.rs             # Pre-computed ATR, ADX
│   └── error.rs                  # Data-specific errors
├── tests/
│   └── integration.rs
```

### 2.2 Yahoo Finance Client

```rust
// crates/trendlab-data/src/client/yahoo.rs

use crate::error::DataError;
use chrono::{NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Yahoo Finance OHLCV response format
#[derive(Debug, Deserialize)]
struct YahooResponse {
    chart: ChartResult,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    result: Vec<ChartData>,
}

#[derive(Debug, Deserialize)]
struct ChartData {
    timestamp: Vec<i64>,
    indicators: Indicators,
}

#[derive(Debug, Deserialize)]
struct Indicators {
    quote: Vec<Quote>,
    adjclose: Option<Vec<AdjClose>>,
}

#[derive(Debug, Deserialize)]
struct Quote {
    open: Vec<Option<f64>>,
    high: Vec<Option<f64>>,
    low: Vec<Option<f64>>,
    close: Vec<Option<f64>>,
    volume: Vec<Option<u64>>,
}

#[derive(Debug, Deserialize)]
struct AdjClose {
    adjclose: Vec<Option<f64>>,
}

/// Yahoo Finance client with rate limiting
pub struct YahooClient {
    client: Client,
    rate_limit_ms: u64,
}

impl YahooClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            rate_limit_ms: 500, // 2 requests per second max
        }
    }

    /// Fetch OHLCV data for a symbol
    pub async fn fetch(
        &self,
        symbol: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<RawBar>, DataError> {
        let start_ts = start.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let end_ts = end.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();

        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval=1d",
            symbol, start_ts, end_ts
        );

        let resp = self.client.get(&url)
            .header("User-Agent", "TrendLab/2.0")
            .send()
            .await
            .map_err(|e| DataError::FetchError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DataError::FetchError(format!(
                "Yahoo returned status {}",
                resp.status()
            )));
        }

        let data: YahooResponse = resp.json().await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        self.parse_response(data)
    }

    fn parse_response(&self, resp: YahooResponse) -> Result<Vec<RawBar>, DataError> {
        let chart = resp.chart.result.into_iter().next()
            .ok_or(DataError::EmptyResponse)?;

        let quote = chart.indicators.quote.into_iter().next()
            .ok_or(DataError::EmptyResponse)?;

        let adj_close = chart.indicators.adjclose
            .and_then(|ac| ac.into_iter().next())
            .map(|ac| ac.adjclose);

        let mut bars = Vec::with_capacity(chart.timestamp.len());

        for (i, &ts) in chart.timestamp.iter().enumerate() {
            let date = chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.date_naive())
                .ok_or(DataError::ParseError("Invalid timestamp".into()))?;

            let open = quote.open.get(i).and_then(|v| *v);
            let high = quote.high.get(i).and_then(|v| *v);
            let low = quote.low.get(i).and_then(|v| *v);
            let close = quote.close.get(i).and_then(|v| *v);
            let volume = quote.volume.get(i).and_then(|v| *v);
            let adj = adj_close.as_ref().and_then(|ac| ac.get(i).and_then(|v| *v));

            if let (Some(o), Some(h), Some(l), Some(c), Some(v)) = (open, high, low, close, volume) {
                bars.push(RawBar {
                    date,
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    volume: v,
                    adj_close: adj,
                });
            }
        }

        Ok(bars)
    }
}

/// Raw bar before indicator computation
#[derive(Debug, Clone)]
pub struct RawBar {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub adj_close: Option<f64>,
}
```

### 2.3 Parquet Caching

```rust
// crates/trendlab-data/src/cache/parquet.rs

use crate::client::RawBar;
use crate::error::DataError;
use chrono::NaiveDate;
use polars::prelude::*;
use std::path::{Path, PathBuf};

/// Cache directory structure:
/// data/
///   ├── ohlcv/
///   │   ├── AAPL.parquet
///   │   ├── MSFT.parquet
///   │   └── ...
///   └── metadata.json

pub struct ParquetCache {
    root: PathBuf,
}

impl ParquetCache {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("ohlcv")).ok();
        Self { root }
    }

    /// Path to symbol's parquet file
    fn symbol_path(&self, symbol: &str) -> PathBuf {
        self.root.join("ohlcv").join(format!("{}.parquet", symbol))
    }

    /// Check if symbol data exists
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.symbol_path(symbol).exists()
    }

    /// Get the date range of cached data
    pub fn date_range(&self, symbol: &str) -> Option<(NaiveDate, NaiveDate)> {
        let path = self.symbol_path(symbol);
        if !path.exists() {
            return None;
        }

        let lf = LazyFrame::scan_parquet(&path, Default::default()).ok()?;

        let stats = lf
            .select([
                col("date").min().alias("min_date"),
                col("date").max().alias("max_date"),
            ])
            .collect()
            .ok()?;

        let min_date = stats.column("min_date").ok()?
            .date()
            .ok()?
            .get(0)?;
        let max_date = stats.column("max_date").ok()?
            .date()
            .ok()?
            .get(0)?;

        Some((
            NaiveDate::from_num_days_from_ce_opt(min_date)?,
            NaiveDate::from_num_days_from_ce_opt(max_date)?,
        ))
    }

    /// Write bars to parquet
    pub fn write(&self, symbol: &str, bars: &[RawBar]) -> Result<(), DataError> {
        let dates: Vec<i32> = bars.iter()
            .map(|b| b.date.num_days_from_ce())
            .collect();
        let opens: Vec<f64> = bars.iter().map(|b| b.open).collect();
        let highs: Vec<f64> = bars.iter().map(|b| b.high).collect();
        let lows: Vec<f64> = bars.iter().map(|b| b.low).collect();
        let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let volumes: Vec<u64> = bars.iter().map(|b| b.volume).collect();

        let df = DataFrame::new(vec![
            Series::new("date".into(), dates).cast(&DataType::Date)?,
            Series::new("open".into(), opens),
            Series::new("high".into(), highs),
            Series::new("low".into(), lows),
            Series::new("close".into(), closes),
            Series::new("volume".into(), volumes),
        ])?;

        let path = self.symbol_path(symbol);
        let file = std::fs::File::create(&path)?;
        ParquetWriter::new(file)
            .with_compression(ParquetCompression::Zstd(None))
            .finish(&mut df.clone())?;

        Ok(())
    }

    /// Read bars as LazyFrame (NEVER eager!)
    pub fn scan(&self, symbol: &str) -> Result<LazyFrame, DataError> {
        let path = self.symbol_path(symbol);
        if !path.exists() {
            return Err(DataError::NotFound(symbol.to_string()));
        }

        LazyFrame::scan_parquet(&path, Default::default())
            .map_err(|e| DataError::ReadError(e.to_string()))
    }

    /// Append new bars (delta update)
    pub fn append(&self, symbol: &str, new_bars: &[RawBar]) -> Result<(), DataError> {
        if new_bars.is_empty() {
            return Ok(());
        }

        let existing = self.scan(symbol)?;

        // Get existing dates
        let existing_dates: std::collections::HashSet<NaiveDate> = existing
            .clone()
            .select([col("date")])
            .collect()?
            .column("date")?
            .date()?
            .into_iter()
            .filter_map(|d| d.and_then(NaiveDate::from_num_days_from_ce_opt))
            .collect();

        // Filter new bars to only those not in existing
        let filtered: Vec<RawBar> = new_bars.iter()
            .filter(|b| !existing_dates.contains(&b.date))
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Ok(());
        }

        // Create DataFrame from new bars
        let dates: Vec<i32> = filtered.iter()
            .map(|b| b.date.num_days_from_ce())
            .collect();
        let opens: Vec<f64> = filtered.iter().map(|b| b.open).collect();
        let highs: Vec<f64> = filtered.iter().map(|b| b.high).collect();
        let lows: Vec<f64> = filtered.iter().map(|b| b.low).collect();
        let closes: Vec<f64> = filtered.iter().map(|b| b.close).collect();
        let volumes: Vec<u64> = filtered.iter().map(|b| b.volume).collect();

        let new_df = DataFrame::new(vec![
            Series::new("date".into(), dates).cast(&DataType::Date)?,
            Series::new("open".into(), opens),
            Series::new("high".into(), highs),
            Series::new("low".into(), lows),
            Series::new("close".into(), closes),
            Series::new("volume".into(), volumes),
        ])?;

        // Concatenate and sort
        let combined = concat([existing, new_df.lazy()], Default::default())?
            .sort(["date"], Default::default())
            .collect()?;

        // Write back
        let path = self.symbol_path(symbol);
        let file = std::fs::File::create(&path)?;
        ParquetWriter::new(file)
            .with_compression(ParquetCompression::Zstd(None))
            .finish(&mut combined.clone())?;

        Ok(())
    }
}
```

### 2.4 Symbol Universe

```rust
// crates/trendlab-data/src/universe.rs

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Symbol universe for backtesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Universe {
    /// Symbols in this universe
    pub symbols: Vec<String>,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: String,
}

impl Universe {
    /// Default universe: Major US equities
    pub fn default_us_equities() -> Self {
        Self {
            symbols: vec![
                // Tech
                "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA",
                // Finance
                "JPM", "BAC", "WFC", "GS", "MS",
                // Healthcare
                "JNJ", "UNH", "PFE", "MRK", "ABBV",
                // Consumer
                "WMT", "PG", "KO", "PEP", "MCD",
                // Industrial
                "CAT", "BA", "GE", "MMM", "HON",
                // Energy
                "XOM", "CVX", "COP",
            ].into_iter().map(String::from).collect(),
            name: "US Major Equities".into(),
            description: "30 major US stocks across sectors".into(),
        }
    }

    /// Futures universe
    pub fn futures() -> Self {
        Self {
            symbols: vec![
                "ES=F", "NQ=F", "YM=F", "RTY=F", // Equity indices
                "GC=F", "SI=F", "CL=F", "NG=F", // Commodities
                "ZB=F", "ZN=F", "ZF=F",          // Bonds
                "6E=F", "6J=F", "6B=F",          // Currencies
            ].into_iter().map(String::from).collect(),
            name: "Futures".into(),
            description: "Major futures contracts".into(),
        }
    }

    /// Create custom universe
    pub fn custom(name: &str, symbols: Vec<String>) -> Self {
        Self {
            symbols,
            name: name.into(),
            description: "Custom universe".into(),
        }
    }

    /// Symbol count
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}
```

### 2.5 Pre-computed Indicators

```rust
// crates/trendlab-data/src/indicators.rs

use polars::prelude::*;

/// Compute ATR (Average True Range) for a LazyFrame
///
/// Adds column "atr_{period}" to the frame
pub fn compute_atr(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("atr_{}", period);

    lf.with_column(
        // True Range = max(high - low, |high - prev_close|, |low - prev_close|)
        when(col("close").shift(lit(1)).is_null())
            .then(col("high") - col("low"))
            .otherwise(
                (col("high") - col("low"))
                    .max(
                        (col("high") - col("close").shift(lit(1))).abs()
                    )
                    .max(
                        (col("low") - col("close").shift(lit(1))).abs()
                    )
            )
            .alias("tr")
    )
    .with_column(
        col("tr")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                ..Default::default()
            })
            .alias(col_name.as_str())
    )
    .drop(["tr"])
}

/// Compute ADX (Average Directional Index) for a LazyFrame
///
/// Adds column "adx_{period}" to the frame
pub fn compute_adx(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("adx_{}", period);

    // Simplified ADX calculation
    // Full ADX requires +DI, -DI, DX calculations
    lf.with_column(
        // Directional Movement
        when(
            (col("high") - col("high").shift(lit(1)))
                .gt(col("low").shift(lit(1)) - col("low"))
        )
        .then(
            (col("high") - col("high").shift(lit(1))).clip_min(lit(0.0))
        )
        .otherwise(lit(0.0))
        .alias("plus_dm")
    )
    .with_column(
        when(
            (col("low").shift(lit(1)) - col("low"))
                .gt(col("high") - col("high").shift(lit(1)))
        )
        .then(
            (col("low").shift(lit(1)) - col("low")).clip_min(lit(0.0))
        )
        .otherwise(lit(0.0))
        .alias("minus_dm")
    )
    .with_column(
        // ATR for smoothing
        when(col("close").shift(lit(1)).is_null())
            .then(col("high") - col("low"))
            .otherwise(
                (col("high") - col("low"))
                    .max((col("high") - col("close").shift(lit(1))).abs())
                    .max((col("low") - col("close").shift(lit(1))).abs())
            )
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                ..Default::default()
            })
            .alias("atr_temp")
    )
    .with_column(
        // Smoothed +DM / ATR * 100
        (col("plus_dm")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                ..Default::default()
            }) / col("atr_temp") * lit(100.0))
            .alias("plus_di")
    )
    .with_column(
        // Smoothed -DM / ATR * 100
        (col("minus_dm")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                ..Default::default()
            }) / col("atr_temp") * lit(100.0))
            .alias("minus_di")
    )
    .with_column(
        // DX = |+DI - -DI| / (+DI + -DI) * 100
        ((col("plus_di") - col("minus_di")).abs()
            / (col("plus_di") + col("minus_di"))
            * lit(100.0))
            .alias("dx")
    )
    .with_column(
        // ADX = Smoothed DX
        col("dx")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                ..Default::default()
            })
            .alias(col_name.as_str())
    )
    .drop(["plus_dm", "minus_dm", "atr_temp", "plus_di", "minus_di", "dx"])
}
```

### 2.6 BDD Scenarios (Phase 2)

```gherkin
# features/data/caching.feature

Feature: Data Caching
  As a user
  I want data to be cached locally
  So that I don't re-fetch on every run

  Scenario: First fetch caches to parquet
    Given symbol "AAPL" has no cached data
    When I fetch AAPL data for 2023
    Then a file data/ohlcv/AAPL.parquet is created
    And it uses ZSTD compression

  Scenario: Subsequent reads use cache
    Given AAPL.parquet exists with data through 2023-06-30
    When I request AAPL data for 2023-01-01 to 2023-06-30
    Then no network request is made
    And data is read from parquet

  Scenario: Delta sync fetches only new data
    Given AAPL.parquet exists with data through 2023-06-30
    When I request AAPL data through 2023-12-31
    Then only 2023-07-01 to 2023-12-31 is fetched
    And new data is appended to parquet
```

```gherkin
# features/data/lazy_loading.feature

Feature: Lazy Data Loading
  As a performance-conscious system
  I want to use lazy Polars operations
  So that memory usage is minimized

  Scenario: Data is scanned lazily
    Given AAPL.parquet with 10 years of data
    When I call cache.scan("AAPL")
    Then a LazyFrame is returned
    And no data is loaded into memory yet

  Scenario: Filtering happens before loading
    Given a LazyFrame for AAPL
    When I filter for date > 2023-01-01
    Then the filter is pushed down to parquet scan
    And only matching rows are loaded
```

### 2.7 Implementation Order (Phase 2)

1. **error.rs** - Data-specific errors
2. **client/yahoo.rs** - Yahoo Finance client
3. **cache/parquet.rs** - Parquet caching
4. **universe.rs** - Symbol universe
5. **indicators.rs** - ATR, ADX computation
6. **lib.rs** - Expose unified API
7. **Integration tests** - End-to-end data flow
8. **BDD scenarios** - Lock behavior

---

## Phase 3: Engine Spine

### 3.1 Backtest Engine

```rust
// crates/trendlab-core/src/engine.rs

use crate::types::*;
use crate::traits::*;
use crate::market_state::MarketState;

/// Backtest engine configuration
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub initial_equity: f64,
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

/// Composed strategy (the four orthogonal layers)
pub struct Strategy {
    pub signal_generator: Box<dyn SignalGenerator>,
    pub position_manager: Box<dyn PositionManager>,
    pub execution_model: Box<dyn ExecutionModel>,
    pub signal_filter: Option<Box<dyn SignalFilter>>,
}

/// Backtest result
#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<f64>,
    pub metrics: Metrics,
    pub fingerprint: String,
}

/// The sacred backtest engine
///
/// # The Event Loop (per bar N)
///
/// 1. Update position tracking (if position exists)
///    - high_since_entry = max(current, bar.high)
///    - low_since_entry = min(current, bar.low)
///    - bars_held += 1
///
/// 2. Check for EXITS BEFORE ENTRIES
///    - Check stop via ExecutionModel (uses prior stop; stop updates apply next bar)
///    - Check PositionManager exit action
///    - Check SignalFilter force exit (document precedence if it should override PM)
///    - If exit: record trade, clear position
///
/// 3. If no position AND past warmup, check for entries
///    - Get signal from SignalGenerator
///    - Check SignalFilter allow
///    - Attempt fill via ExecutionModel (using bar N+1)
///    - If filled: create position, call PM.on_entry()
///
/// 4. Record equity snapshot (per bar, consistent length with bars)
pub struct BacktestEngine {
    config: BacktestConfig,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }

    pub fn run(
        &self,
        strategy: &mut Strategy,
        bars: &[Bar],
        atr: &[f64],
        adx: &[f64],
    ) -> BacktestResult {
        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<f64> = Vec::with_capacity(bars.len());
        let mut position: Option<Position> = None;
        let mut equity = self.config.initial_equity;

        let warmup = strategy.signal_generator.warmup_bars();

        for i in 0..bars.len() {
            let bar = &bars[i];

            // Create market state (ONLY bars up to current)
            let state = MarketState {
                bars: &bars[..=i],
                current_idx: i,
                atr: &atr[..=i],
                adx: &adx[..=i],
            };

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

                // 2b. Check PositionManager action (if no stop)
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
                    let trade = Self::create_trade(&pos, i, bar.date, price, reason);
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
                    let allowed = strategy.signal_filter
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
                        let fill = strategy.execution_model.attempt_fill(&order, bar, fill_bar);

                        if fill.filled {
                            strategy.position_manager.on_entry(fill_bar, fill.fill_price, &signal);

                            position = Some(Position {
                                entry_bar_idx: fill.fill_bar_idx,
                                entry_date: fill_bar.date,
                                entry_price: fill.fill_price,
                                direction: signal.direction,
                                size: order.size,
                                signal,
                                high_since_entry: fill.fill_price,
                                low_since_entry: fill.fill_price,
                                bars_held: 0,
                                stop_price: strategy.position_manager.stop_price(),
                            });
                        }
                    }
                }
            }

            // === STEP 4: Record equity ===
            equity_curve.push(equity);
        }

        // Close any remaining position
        if let Some(pos) = position {
            let last_bar = bars.last().unwrap();
            let trade = Self::create_trade(
                &pos,
                bars.len() - 1,
                last_bar.date,
                last_bar.close,
                ExitReason::EndOfData,
            );
            equity *= 1.0 + trade.return_pct;
            if let Some(last) = equity_curve.last_mut() {
                *last = equity;
            } else {
                equity_curve.push(equity);
            }
            trades.push(trade);
        }

        let metrics = Self::calculate_metrics(&trades, &equity_curve, bars.len());

        BacktestResult {
            trades,
            equity_curve,
            metrics,
            fingerprint: String::new(), // Computed by caller
        }
    }

    fn create_trade(
        pos: &Position,
        exit_idx: usize,
        exit_date: chrono::NaiveDate,
        exit_price: f64,
        reason: ExitReason,
    ) -> Trade {
        let return_pct = match pos.direction {
            Direction::Long => (exit_price - pos.entry_price) / pos.entry_price,
            Direction::Short => (pos.entry_price - exit_price) / pos.entry_price,
        };

        let (mae, mfe) = match pos.direction {
            Direction::Long => {
                ((pos.low_since_entry - pos.entry_price) / pos.entry_price,
                 (pos.high_since_entry - pos.entry_price) / pos.entry_price)
            }
            Direction::Short => {
                ((pos.entry_price - pos.high_since_entry) / pos.entry_price,
                 (pos.entry_price - pos.low_since_entry) / pos.entry_price)
            }
        };

        Trade {
            entry_bar_idx: pos.entry_bar_idx,
            entry_date: pos.entry_date,
            entry_price: pos.entry_price,
            exit_bar_idx: exit_idx,
            exit_date,
            exit_price,
            direction: pos.direction,
            size: pos.size,
            exit_reason: reason,
            return_pct,
            bars_held: pos.bars_held,
            mae,
            mfe,
        }
    }

    fn calculate_metrics(
        trades: &[Trade],
        equity_curve: &[f64],
        total_bars: usize,
    ) -> Metrics {
        if trades.is_empty() {
            return Metrics::default();
        }

        let initial = equity_curve.first().copied().unwrap_or(100_000.0);
        let final_eq = equity_curve.last().copied().unwrap_or(initial);
        let total_return = (final_eq / initial) - 1.0;

        let years = total_bars as f64 / 252.0;
        let cagr = if years > 0.0 {
            (final_eq / initial).powf(1.0 / years) - 1.0
        } else {
            0.0
        };

        // Daily returns
        let daily_returns: Vec<f64> = equity_curve
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();

        let mean_return = if !daily_returns.is_empty() {
            daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
        } else {
            0.0
        };

        let variance = if !daily_returns.is_empty() {
            daily_returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / daily_returns.len() as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        let sharpe = if std_dev > 0.0 {
            (mean_return / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // Sortino
        let downside: Vec<f64> = daily_returns.iter()
            .filter(|&&r| r < 0.0)
            .copied()
            .collect();
        let downside_std = if !downside.is_empty() {
            (downside.iter().map(|r| r.powi(2)).sum::<f64>() / downside.len() as f64).sqrt()
        } else {
            0.0
        };
        let sortino = if downside_std > 0.0 {
            (mean_return / downside_std) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // Max drawdown
        let mut peak = initial;
        let mut max_dd = 0.0;
        for &eq in equity_curve {
            peak = peak.max(eq);
            let dd = (peak - eq) / peak;
            max_dd = max_dd.max(dd);
        }

        // Trade stats
        let winners: Vec<_> = trades.iter().filter(|t| t.return_pct > 0.0).collect();
        let losers: Vec<_> = trades.iter().filter(|t| t.return_pct < 0.0).collect();

        let win_rate = winners.len() as f64 / trades.len() as f64;

        let gross_profit: f64 = winners.iter().map(|t| t.return_pct).sum();
        let gross_loss: f64 = losers.iter().map(|t| t.return_pct.abs()).sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else {
            f64::INFINITY
        };

        let avg_return = trades.iter().map(|t| t.return_pct).sum::<f64>() / trades.len() as f64;
        let avg_winner = if !winners.is_empty() {
            winners.iter().map(|t| t.return_pct).sum::<f64>() / winners.len() as f64
        } else {
            0.0
        };
        let avg_loser = if !losers.is_empty() {
            losers.iter().map(|t| t.return_pct).sum::<f64>() / losers.len() as f64
        } else {
            0.0
        };
        let avg_bars_held = trades.iter().map(|t| t.bars_held as f64).sum::<f64>() / trades.len() as f64;

        Metrics {
            total_return,
            cagr,
            sharpe,
            sortino,
            max_drawdown: max_dd,
            win_rate,
            profit_factor,
            total_trades: trades.len(),
            avg_bars_held,
            avg_return,
            avg_winner,
            avg_loser,
        }
    }
}
```

### 3.2 First SignalGenerator: Donchian Breakout

```rust
// crates/trendlab-core/src/signals/donchian.rs

use crate::types::{Bar, Signal, Direction};
use crate::traits::SignalGenerator;
use crate::market_state::MarketState;
use crate::param::{ParamDef, ParamType};

/// Donchian Channel Breakout Signal Generator
///
/// Generates long signals when price breaks above the N-period high.
/// Generates short signals when price breaks below the N-period low.
#[derive(Debug, Clone)]
pub struct DonchianBreakout {
    lookback: usize,
    long_only: bool,
}

impl DonchianBreakout {
    pub fn new(lookback: usize, long_only: bool) -> Self {
        Self { lookback, long_only }
    }
}

impl SignalGenerator for DonchianBreakout {
    fn name(&self) -> &str {
        "DonchianBreakout"
    }

    fn warmup_bars(&self) -> usize {
        self.lookback
    }

    fn generate(&self, bar: &Bar, state: &MarketState) -> Option<Signal> {
        if state.current_idx < self.lookback {
            return None;
        }

        let highest = state.highest_high(self.lookback);
        let lowest = state.lowest_low(self.lookback);

        // Long breakout
        if bar.close > highest {
            let raw_strength = (bar.close - highest) / highest;
            return Some(Signal {
                direction: Direction::Long,
                entry_level: None,
                strength: raw_strength.clamp(0.0, 1.0),
                trigger_value: highest,
            });
        }

        // Short breakout (if enabled)
        if !self.long_only && bar.close < lowest {
            let raw_strength = (lowest - bar.close) / lowest;
            return Some(Signal {
                direction: Direction::Short,
                entry_level: None,
                strength: raw_strength.clamp(0.0, 1.0),
                trigger_value: lowest,
            });
        }

        None
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "lookback".into(),
                param_type: ParamType::Int { min: 10, max: 252, step: 5 },
            },
            ParamDef {
                name: "long_only".into(),
                param_type: ParamType::Bool,
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn SignalGenerator> {
        Box::new(self.clone())
    }
}
```

### 3.3 First PositionManager: ATR Trailing Stop

```rust
// crates/trendlab-core/src/position_managers/atr_trailing.rs

use crate::types::{Bar, Position, Signal, Action, ExitReason, Direction};
use crate::traits::PositionManager;
use crate::market_state::MarketState;
use crate::exit_reference::ExitReferenceMode;
use crate::param::{ParamDef, ParamType};

/// ATR Trailing Stop Position Manager
///
/// Tracks the highest price SINCE ENTRY and places a stop at
/// `high_since_entry - (atr_multiplier * ATR)`.
///
/// Exit reference mode: SinceEntryTrailingExtreme
#[derive(Debug, Clone)]
pub struct AtrTrailingStop {
    atr_multiplier: f64,
    high_since_entry: f64,
    stop_price: f64,
}

impl AtrTrailingStop {
    pub fn new(atr_multiplier: f64) -> Self {
        Self {
            atr_multiplier,
            high_since_entry: 0.0,
            stop_price: 0.0,
        }
    }
}

impl PositionManager for AtrTrailingStop {
    fn name(&self) -> &str {
        "ATRTrailingStop"
    }

    fn exit_reference_mode(&self) -> Option<ExitReferenceMode> {
        Some(ExitReferenceMode::SinceEntryTrailingExtreme)
    }

    fn on_entry(&mut self, entry_bar: &Bar, entry_price: f64, _signal: &Signal) {
        // CRITICAL: Initialize from entry, NOT historical!
        self.high_since_entry = entry_price;

        let atr = entry_bar.high - entry_bar.low; // Proxy for ATR at entry (consider passing ATR explicitly)
        self.stop_price = entry_price - (self.atr_multiplier * atr);
    }

    fn on_bar(&mut self, bar: &Bar, position: &Position, state: &MarketState) -> Action {
        let atr = state.current_atr();

        // Update from engine-tracked position
        self.high_since_entry = position.high_since_entry;

        // Calculate new trailing stop
        let new_stop = match position.direction {
            Direction::Long => self.high_since_entry - (self.atr_multiplier * atr),
            Direction::Short => position.low_since_entry + (self.atr_multiplier * atr),
        };

        // Only ratchet up for longs
        if new_stop > self.stop_price {
            self.stop_price = new_stop;
        }

        // Check stop hit
        let stop_hit = match position.direction {
            Direction::Long => bar.low <= self.stop_price,
            Direction::Short => bar.high >= self.stop_price,
        };

        if stop_hit {
            Action::Exit(ExitReason::StopHit)
        } else {
            Action::AdjustStop(self.stop_price)
        }
    }

    fn stop_price(&self) -> Option<f64> {
        Some(self.stop_price)
    }

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "atr_multiplier".into(),
                param_type: ParamType::Float { min: 1.0, max: 5.0, step: 0.5 },
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn PositionManager> {
        Box::new(AtrTrailingStop::new(self.atr_multiplier))
    }
}
```

### 3.4 First ExecutionModel: Next Open Fill

```rust
// crates/trendlab-core/src/execution/next_open.rs

use crate::types::{Bar, Order, Position, FillResult, GapPolicy, Direction};
use crate::traits::ExecutionModel;
use crate::param::{ParamDef, ParamType};

/// Next Open Fill Execution Model
///
/// - Entry: Fill at next bar's open price
/// - Stop: Check if bar breaches stop; fill at open if gap through
#[derive(Debug, Clone)]
pub struct NextOpenFill {
    commission_pct: f64,
    slippage_pct: f64,
}

impl NextOpenFill {
    pub fn new(commission_pct: f64, slippage_pct: f64) -> Self {
        Self { commission_pct, slippage_pct }
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

        let slippage = match order.direction {
            Direction::Long => base_price * self.slippage_pct,
            Direction::Short => -base_price * self.slippage_pct,
        };

        let fill_price = base_price + slippage;
        let commission = fill_price * self.commission_pct;

        FillResult {
            filled: true,
            fill_price,
            fill_bar_idx: fill_bar.idx,
            slippage,
            commission,
        }
    }

    fn check_stop(
        &self,
        position: &Position,
        bar: &Bar,
    ) -> Option<f64> {
        let stop = position.stop_price?;

        match position.direction {
            Direction::Long => {
                if bar.low <= stop {
                    if bar.open <= stop {
                        Some(bar.open) // Gap through
                    } else {
                        Some(stop) // Normal hit
                    }
                } else {
                    None
                }
            }
            Direction::Short => {
                if bar.high >= stop {
                    if bar.open >= stop {
                        Some(bar.open) // Gap through
                    } else {
                        Some(stop) // Normal hit
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

    fn parameter_spec(&self) -> Vec<ParamDef> {
        vec![
            ParamDef {
                name: "commission_pct".into(),
                param_type: ParamType::Float { min: 0.0, max: 0.01, step: 0.001 },
            },
            ParamDef {
                name: "slippage_pct".into(),
                param_type: ParamType::Float { min: 0.0, max: 0.005, step: 0.0005 },
            },
        ]
    }

    fn box_clone(&self) -> Box<dyn ExecutionModel> {
        Box::new(self.clone())
    }
}
```

### 3.5 BDD Scenarios (Phase 3)

```gherkin
# features/engine/trade_lifecycle.feature

Feature: Trade Lifecycle
  As a backtester
  I want trades to follow a predictable lifecycle
  So that results are reproducible and correct

  Background:
    Given a Donchian breakout signal generator with lookback 20
    And an ATR trailing stop position manager with multiplier 3.0
    And a next-open execution model

  Scenario: Signal generates fill at next open
    Given bars from "trending_up_50bars.csv"
    And bar 25 closes above the 20-bar high
    When the engine processes bar 25
    Then a long signal is generated
    And fill occurs at bar 26 open price

  Scenario: Exit checks occur before entry checks
    Given an open long position from bar 10
    And bar 20 triggers BOTH stop hit AND new entry signal
    When the engine processes bar 20
    Then the existing position is exited FIRST
    And no new entry on bar 20
    And entry may occur on bar 21

  Scenario: Stop hit fills correctly
    Given an open long position with stop at $95
    And bar: open $98, low $94, close $96
    When stop check runs
    Then stop is hit at $95 (the stop price)

  Scenario: Gap through stop fills at open
    Given an open long position with stop at $95
    And bar: open $93, low $92, close $94
    When stop check runs
    Then stop is hit at $93 (the open, gap through)
```

```gherkin
# features/engine/determinism.feature

Feature: Determinism
  As a researcher
  I want identical inputs to produce identical outputs
  So that results are reproducible

  Scenario: Same config produces same results
    Given strategy config "donchian_20_atr_3"
    And bars from "aapl_test_data.parquet"
    When backtest runs twice
    Then trade lists are identical
    And equity curves are identical
    And all metrics match exactly

  Scenario: Fresh PM state per run
    Given PM used in run 1 with high_since_entry=$150
    When run 2 starts with same PM type
    Then PM state is fresh
    And high_since_entry starts from entry price, not $150
```

### 3.6 Implementation Order (Phase 3)

1. **engine.rs** - Core backtest loop (most critical!)
2. **signals/donchian.rs** - First SignalGenerator
3. **position_managers/atr_trailing.rs** - First PositionManager
4. **execution/next_open.rs** - First ExecutionModel
5. **BDD step definitions** - Lock behavior
6. **Determinism tests** - Verify reproducibility
7. **Integration tests** - Full backtest round-trips

---

## Phase 4: YOLO Discovery

### 4.1 Genome Representation

```rust
// crates/trendlab-yolo/src/genome.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use trendlab_core::param::ParamValue;

/// Genome encodes a complete strategy composition + parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub id: String,
    pub components: ComponentSelection,
    pub parameters: ComponentParameters,
    pub generation: usize,
    pub parent_id: Option<String>,
    pub origin: GenomeOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSelection {
    pub signal_generator: String,
    pub position_manager: String,
    pub execution_model: String,
    pub signal_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentParameters {
    pub signal_generator: HashMap<String, ParamValue>,
    pub position_manager: HashMap<String, ParamValue>,
    pub execution_model: HashMap<String, ParamValue>,
    pub signal_filter: Option<HashMap<String, ParamValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenomeOrigin {
    Warmup,
    StructuralMutation { mutated_component: String },
    ParameterJitter { jittered_params: Vec<String> },
    Crossover { parent_a: String, parent_b: String },
}

impl Genome {
    pub fn fingerprint(&self) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.components.signal_generator.hash(&mut hasher);
        self.components.position_manager.hash(&mut hasher);
        self.components.execution_model.hash(&mut hasher);
        // ... hash all parameters ...
        format!("{:016x}", hasher.finish())
    }

    pub fn structural_signature(&self) -> String {
        format!(
            "{}+{}+{}+{}",
            self.components.signal_generator,
            self.components.position_manager,
            self.components.execution_model,
            self.components.signal_filter.as_deref().unwrap_or("none")
        )
    }
}
```

### 4.2 Robustness Scoring

```rust
// crates/trendlab-yolo/src/robustness.rs

use std::collections::HashMap;
use trendlab_core::engine::BacktestResult;

/// Robustness scoring configuration
#[derive(Debug, Clone)]
pub struct RobustnessConfig {
    pub median_sharpe_weight: f64,
    pub hit_rate_weight: f64,
    pub consistency_weight: f64, // Negative = penalty
    pub floor_weight: f64,
    pub drawdown_penalty: f64,
    pub fragility_penalty: f64,
}

impl Default for RobustnessConfig {
    fn default() -> Self {
        Self {
            median_sharpe_weight: 0.40,
            hit_rate_weight: 0.30,
            consistency_weight: -0.20,
            floor_weight: 0.10,
            drawdown_penalty: 0.5,
            fragility_penalty: 0.3,
        }
    }
}

/// Results across the symbol universe
pub struct UniverseResults {
    pub by_symbol: HashMap<String, BacktestResult>,
    pub cost_sensitivity: f64,
}

/// Score robustness from universe results
///
/// Formula:
/// ```text
/// base = (
///     w_median * median_sharpe +
///     w_hit * hit_rate +
///     w_consistency * -std_sharpe +
///     w_floor * floor_sharpe
/// )
/// robustness = base * (1 - dd_penalty * max_dd) * (1 - fragility * cost_sens)
/// ```
pub fn score_robustness(results: &UniverseResults, config: &RobustnessConfig) -> f64 {
    let sharpes: Vec<f64> = results.by_symbol.values()
        .map(|r| r.metrics.sharpe)
        .collect();

    if sharpes.is_empty() {
        return 0.0;
    }

    let mut sorted = sharpes.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Median
    let median = sorted[sorted.len() / 2];

    // Hit rate
    let positive = sharpes.iter().filter(|&&s| s > 0.0).count();
    let hit_rate = positive as f64 / sharpes.len() as f64;

    // Consistency (std dev)
    let mean = sharpes.iter().sum::<f64>() / sharpes.len() as f64;
    let variance = sharpes.iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f64>() / sharpes.len() as f64;
    let std_dev = variance.sqrt();

    // Floor (worst)
    let floor = sorted[0];

    // Max drawdown
    let max_dd = results.by_symbol.values()
        .map(|r| r.metrics.max_drawdown)
        .fold(0.0, f64::max);

    // Base score
    let base = config.median_sharpe_weight * median
        + config.hit_rate_weight * hit_rate
        + config.consistency_weight * std_dev
        + config.floor_weight * floor;

    // Apply penalties
    let dd_mult = 1.0 - config.drawdown_penalty * max_dd;
    let frag_mult = 1.0 - config.fragility_penalty * results.cost_sensitivity;

    (base * dd_mult * frag_mult).clamp(-1.0, 2.0)
}

/// Lower-tail emphasis scoring
pub fn score_tail_emphasis(results: &UniverseResults) -> f64 {
    let mut sharpes: Vec<f64> = results.by_symbol.values()
        .map(|r| r.metrics.sharpe)
        .collect();

    if sharpes.is_empty() {
        return 0.0;
    }

    sharpes.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p10_idx = (sharpes.len() as f64 * 0.10).floor() as usize;
    let p25_idx = (sharpes.len() as f64 * 0.25).floor() as usize;
    let p50_idx = sharpes.len() / 2;

    let p10 = sharpes.get(p10_idx).copied().unwrap_or(sharpes[0]);
    let p25 = sharpes.get(p25_idx).copied().unwrap_or(sharpes[0]);
    let p50 = sharpes[p50_idx];

    // Weight lower tail more heavily
    0.40 * p10 + 0.30 * p25 + 0.30 * p50
}
```

### 4.3 Three Leaderboards

```rust
// crates/trendlab-yolo/src/leaderboard.rs

use crate::genome::Genome;
use crate::robustness::UniverseResults;
use std::collections::BinaryHeap;

#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub genome: Genome,
    pub results: UniverseResults,
    pub robustness: f64,
}

/// Three disentangled leaderboards (Invariant E)
pub struct LeaderboardSet {
    /// Best signal generators (fixed PM + Execution)
    pub signal_quality: Leaderboard,

    /// Best position managers (fixed Signal + Execution)
    pub position_management: Leaderboard,

    /// Execution sensitivity (fixed Signal + PM)
    pub execution_sensitivity: Leaderboard,

    /// Overall best compositions
    pub overall: Leaderboard,

    baseline: BaselineComponents,
}

#[derive(Debug, Clone)]
struct BaselineComponents {
    signal_generator: String,
    position_manager: String,
    execution_model: String,
}

impl LeaderboardSet {
    pub fn new() -> Self {
        Self {
            signal_quality: Leaderboard::new(),
            position_management: Leaderboard::new(),
            execution_sensitivity: Leaderboard::new(),
            overall: Leaderboard::new(),
            baseline: BaselineComponents {
                signal_generator: "DonchianBreakout".into(),
                position_manager: "ATRTrailingStop".into(),
                execution_model: "NextOpenFill".into(),
            },
        }
    }

    pub fn insert(&mut self, entry: LeaderboardEntry) {
        // Always update overall
        self.overall.insert(entry.clone());

        let comps = &entry.genome.components;

        // Signal Quality: PM and EM match baseline
        if comps.position_manager == self.baseline.position_manager
            && comps.execution_model == self.baseline.execution_model
        {
            self.signal_quality.insert(entry.clone());
        }

        // Position Management: SG and EM match baseline
        if comps.signal_generator == self.baseline.signal_generator
            && comps.execution_model == self.baseline.execution_model
        {
            self.position_management.insert(entry.clone());
        }

        // Execution Sensitivity: SG and PM match baseline
        if comps.signal_generator == self.baseline.signal_generator
            && comps.position_manager == self.baseline.position_manager
        {
            self.execution_sensitivity.insert(entry.clone());
        }
    }
}

pub struct Leaderboard {
    entries: Vec<LeaderboardEntry>,
    max_size: usize,
}

impl Leaderboard {
    pub fn new() -> Self {
        Self { entries: Vec::new(), max_size: 100 }
    }

    pub fn insert(&mut self, entry: LeaderboardEntry) {
        self.entries.push(entry);
        self.entries.sort_by(|a, b|
            b.robustness.partial_cmp(&a.robustness).unwrap()
        );
        self.entries.truncate(self.max_size);
    }

    pub fn top(&self, n: usize) -> &[LeaderboardEntry] {
        &self.entries[..n.min(self.entries.len())]
    }
}
```

### 4.4 BDD Scenarios (Phase 4)

```gherkin
# features/yolo/sampling.feature

Feature: YOLO Sampling
  As a researcher
  I want structural sampling before parameter jitter
  So that I explore the full composition space

  Scenario: Warmup samples uniformly
    Given warmup_iterations = 200
    When warmup completes
    Then each signal generator used ~18 times
    And each position manager used ~20 times

  Scenario: Exploitation weights by robustness
    Given genome A has robustness 0.8
    And genome B has robustness 0.4
    When 100 exploitation samples taken
    Then A's structure sampled ~2x more than B's

  Scenario: Structural mutation swaps one component
    Given parent with "Donchian+ATRTrailing+NextOpen"
    When structural mutation applied
    Then exactly ONE component changes
```

```gherkin
# features/yolo/leaderboards.feature

Feature: Three Leaderboards
  As a researcher
  I want to know which COMPONENT drives performance
  So that I don't confuse signal quality with exit quality

  Scenario: Signal Quality leaderboard isolates signals
    Given baseline PM="ATRTrailingStop", EM="NextOpenFill"
    And genome with different SignalGenerator
    When results inserted
    Then Signal Quality leaderboard is updated
    And comparison is fair (PM and EM held constant)

  Scenario: Three leaderboards before trusting overall
    Given YOLO session completes
    Then signal_quality leaderboard has entries
    And position_management leaderboard has entries
    And execution_sensitivity leaderboard has entries
    Then and only then trust overall leaderboard
```

### 4.5 Implementation Order (Phase 4)

1. **genome.rs** - Genome representation
2. **registry.rs** - Component registry
3. **robustness.rs** - Scoring formulas
4. **leaderboard.rs** - Three leaderboards
5. **sampler.rs** - Structural Monte Carlo
6. **session.rs** - YOLO session state machine
7. **attribution.rs** - Component attribution
8. **BDD scenarios** - Lock YOLO behavior

---

## Phase 5: Terminal UI

### 5.1 App State

```rust
// crates/trendlab-tui/src/app.rs

use tokio::sync::mpsc;

/// Central application state
pub struct App {
    pub focused_panel: PanelId,
    pub home: HomeState,
    pub results: ResultsState,
    pub chart: ChartState,
    pub modal: Option<Modal>,
    pub yolo_session: Option<YoloSession>,
    pub yolo_rx: Option<mpsc::UnboundedReceiver<YoloEvent>>,
    pub status_bar: StatusBar,
    pub pending_keys: PendingKeys,
    pub should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Home,    // 1
    Results, // 2
    Chart,   // 3
}

pub struct HomeState {
    pub universe: Universe,
    pub data_status: DataStatus,
    pub yolo_config: YoloConfig,
}

pub struct ResultsState {
    pub leaderboard_view: LeaderboardView,
    pub selected_idx: usize,
    pub scroll_offset: usize,
}

pub struct ChartState {
    pub selected_genome: Option<Genome>,
    pub display_mode: ChartMode,
}

pub enum Modal {
    Help,
    Config,
    Export,
}
```

### 5.2 Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down in list |
| `k` / `↑` | Move up in list |
| `h` / `←` | Previous panel |
| `l` / `→` | Next panel |
| `gg` | Go to top |
| `G` | Go to bottom |
| `1` | Focus Home panel |
| `2` | Focus Results panel |
| `3` | Focus Chart panel |
| `Enter` | Start/stop YOLO |
| `P` | Export selected to Pine |
| `?` | Toggle Help overlay |
| `c` | Open Config modal |
| `q` / `Esc` | Quit / Close modal |

### 5.3 Semantic Colors

| Color | Meaning |
|-------|---------|
| Green | Positive (profit, success, active) |
| Red | Negative (loss, error, stopped) |
| Yellow | Warning, attention needed |
| Blue | Informational, neutral |
| Cyan | Selected, highlighted |
| White | Normal text |
| Gray | Disabled, secondary |

### 5.4 Panel Structure

```
┌─Home─────────────┬─Results──────────────────────┬─Chart─────────────┐
│ Universe: US30   │ Rank │ Signal    │ PM      │ │ [Equity Curve]    │
│ Symbols: 30      │ ───────────────────────────│ │                   │
│ Date: 2020-2024  │ 1    │ Donchian  │ ATRTr.  │ │                   │
│                  │ 2    │ MACross   │ Fixed   │ │ [Trade Markers]   │
│ YOLO Status:     │ 3    │ TSMOM     │ ATRTr.  │ │                   │
│ ● Running 45/500 │ 4    │ Donchian  │ Chand.  │ │                   │
│                  │ 5    │ ATH       │ Time    │ │                   │
│ [Enter] to start │ ────────────────────────────│ │ Sharpe: 0.45      │
│ [?] for help     │ Sharpe │ DD    │ Robust  │ │ DD: -12%          │
│                  │  0.52  │ -8%   │  0.78   │ │ Trades: 42        │
└──────────────────┴──────────────────────────────┴───────────────────┘
```

### 5.5 Implementation Order (Phase 5)

1. **app.rs** - Central state
2. **event.rs** - Event handling
3. **keybinds.rs** - Vim keybindings
4. **colors.rs** - Semantic color system
5. **panels/home.rs** - Home panel
6. **panels/results.rs** - Results panel
7. **panels/chart.rs** - Chart panel
8. **modals/help.rs** - Help overlay
9. **main.rs** - Render loop

---

## Phase 6: Export & Pine Parity

### 6.1 Strategy Artifact

The `StrategyArtifact` JSON matches the schema in `schemas/strategy-artifact.schema.json`:

```json
{
  "schema_version": "1.0.0",
  "strategy_id": "donchian_20_atr_3_abc123",
  "exported_at": "2024-01-15T10:30:00Z",
  "signal_generator": {
    "type": "DonchianBreakout",
    "parameters": { "lookback": 20, "long_only": true },
    "exit_reference_mode": null
  },
  "position_manager": {
    "type": "ATRTrailingStop",
    "parameters": { "atr_multiplier": 3.0 },
    "exit_reference_mode": "SinceEntryTrailingExtreme"
  },
  "execution_model": {
    "type": "NextOpenFill",
    "parameters": { "commission_pct": 0.001, "slippage_pct": 0.0005 }
  },
  "signal_filter": null,
  "backtest_results": {
    "symbol": "AAPL",
    "start_date": "2020-01-01",
    "end_date": "2023-12-31",
    "metrics": { /* ... */ }
  },
  "parity_vectors": {
    "entries": [...],
    "exits": [...],
    "trade_returns": [...]
  },
  "run_fingerprint": "abc123def456"
}
```

### 6.2 Pine Script Generation

```rust
// crates/trendlab-export/src/pine.rs

use crate::artifact::StrategyArtifact;

pub fn generate_pine_v6(artifact: &StrategyArtifact) -> String {
    let mut pine = String::new();

    // Header
    pine.push_str(&format!(
        r#"//@version=6
strategy("{}", overlay=true, initial_capital=100000)

// Generated by TrendLab v2
// Fingerprint: {}
// Exported: {}

"#,
        artifact.strategy_id,
        artifact.run_fingerprint,
        artifact.exported_at,
    ));

    // Signal Generator
    match artifact.signal_generator.type_name.as_str() {
        "DonchianBreakout" => {
            let lookback = artifact.signal_generator.parameters
                .get("lookback")
                .and_then(|v| v.as_i64())
                .unwrap_or(20);
            pine.push_str(&format!(
                r#"// Signal Generator: Donchian Breakout
lookback = {}
highest_high = ta.highest(high, lookback)[1]
lowest_low = ta.lowest(low, lookback)[1]
long_signal = close > highest_high
short_signal = close < lowest_low

"#,
                lookback
            ));
        }
        // ... other signal generators ...
        _ => {}
    }

    // Position Manager
    match artifact.position_manager.type_name.as_str() {
        "ATRTrailingStop" => {
            let mult = artifact.position_manager.parameters
                .get("atr_multiplier")
                .and_then(|v| v.as_f64())
                .unwrap_or(3.0);
            pine.push_str(&format!(
                r#"// Position Manager: ATR Trailing Stop
atr_mult = {}
atr_val = ta.atr(14)

var float high_since_entry = na
var float trail_stop = na

if strategy.position_size > 0
    if na(high_since_entry)
        high_since_entry := high
    else
        high_since_entry := math.max(high_since_entry, high)
    trail_stop := high_since_entry - atr_mult * atr_val

if strategy.position_size <= 0
    high_since_entry := na
    trail_stop := na

"#,
                mult
            ));
        }
        _ => {}
    }

    // Entry/Exit Logic
    pine.push_str(r#"// Entries
if long_signal and strategy.position_size == 0
    strategy.entry("Long", strategy.long)

// Exits
if strategy.position_size > 0 and not na(trail_stop)
    strategy.exit("Stop", "Long", stop=trail_stop)
"#);

    pine
}
```

### 6.3 Parity Testing

```gherkin
# features/export/parity.feature

Feature: Pine Parity
  As a trader
  I want Pine scripts to match Rust backtests
  So that I can trust TradingView results

  Scenario: Entry dates match
    Given strategy exported to Pine
    When Pine runs on TradingView
    Then entry dates match within 1 bar tolerance

  Scenario: Exit prices match
    Given strategy with ATR trailing stop
    When comparing Rust vs Pine exits
    Then exit prices match within 0.1% tolerance

  Scenario: Total return matches
    Given identical data and config
    When comparing Rust total_return vs Pine strategy.netprofit
    Then values match within 1% tolerance
```

### 6.4 Implementation Order (Phase 6)

1. **artifact.rs** - StrategyArtifact struct
2. **pine.rs** - Pine Script generation
3. **parity.rs** - Parity test vectors
4. **export.rs** - Export orchestration
5. **BDD scenarios** - Parity tests

---

## Verification Checklist

### Phase 1
- [ ] `cargo test -p trendlab-core` passes
- [ ] Property tests for state isolation pass
- [ ] No lookahead in MarketState verified

### Phase 2
- [ ] Yahoo fetch works for test symbol
- [ ] Parquet cache creates ZSTD files
- [ ] LazyFrame scans (not eager reads)

### Phase 3
- [ ] Full backtest produces trades
- [ ] Determinism: same input = same output
- [ ] Exit before entry enforced

### Phase 4
- [ ] YOLO samples structures first
- [ ] Three leaderboards populated
- [ ] Robustness scoring applied

### Phase 5
- [ ] `cargo run` shows TUI
- [ ] Vim keys work (j/k/h/l)
- [ ] Enter starts YOLO

### Phase 6
- [ ] Export produces valid JSON
- [ ] Pine script compiles in TradingView
- [ ] Parity within tolerance

---

*Last updated: 2026-01-21*
