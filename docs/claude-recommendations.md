Let me search our past conversations to see if there's additional context about the stickiness issue or other architectural challenges we discussed:Excellent - I've got the full picture now, including our previous deep dive on the stickiness problem. Let me put together a comprehensive architectural plan for TrendLab v2.

---

# TrendLab v2: Architectural Plan

## The Core Insight From v1

The stickiness problem wasn't just a bug—it was a symptom of **architectural conflation**. When 52-Week Breakout used the same rolling reference for both "when to enter" and "when to exit," it created structural bias that no amount of parameter tuning could fix. Supertrend won not because it was a better trend-following method, but because it happened to ship with ATR-adaptive position management baked in.

**The lesson:** Signal generation and position management are **orthogonal concerns** that must be independently composable.

---

## Part 1: Architectural Foundation

### 1.1 The Composition Principle

The new architecture treats every strategy as a **composition of four independent layers**:

```gherkin
Feature: Strategy Composition Architecture
  Every strategy is assembled from orthogonal components.
  No component knows about or depends on any other component's internals.

  Background:
    Given the four component types:
      | Layer             | Responsibility                          |
      | SignalGenerator   | "Should I consider entering right now?" |
      | PositionManager   | "How do I manage this trade once in?"   |
      | ExecutionModel    | "How does my order get filled?"         |
      | SignalFilter      | "Should I suppress this signal?"        |

  Scenario: Clean separation prevents stickiness
    Given a SignalGenerator that uses a 52-week rolling high
    And a PositionManager that uses ATR trailing from entry high
    Then the signal's rolling reference does NOT affect the exit logic
    And the position manager tracks its OWN reference point (entry high)
    And the two components cannot "leak" state into each other

  Scenario: Composition creates the search space
    Given 11 SignalGenerators in the pool
    And 10 PositionManagers in the pool
    And 4 ExecutionModels in the pool
    And 4 SignalFilters in the pool
    Then the structural search space is 11 × 10 × 4 × 4 = 1,760 combinations
    And parameter ranges create continuous sub-spaces within each structure
```

### 1.2 State Isolation (The Stickiness Fix)

```gherkin
Feature: State Isolation Between Components
  Each component maintains its own state.
  Position managers specifically track state SINCE ENTRY, not globally.

  Scenario: Position manager owns its reference point
    Given I enter a long position at $100 on bar N
    And the PositionManager is "SinceEntryTrailingHigh"
    Then the manager initializes its high_since_entry = $100
    When price reaches $115 on bar N+5
    Then high_since_entry updates to $115
    And the trailing stop is now 0.9 × $115 = $103.50
    And this reference is COMPLETELY INDEPENDENT of the signal's lookback

  Scenario: Signal generator has no exit responsibility
    Given a FiftyTwoWeekBreakout signal generator
    Then it provides ONLY: should_enter(bar) -> Option<Signal>
    And it does NOT provide: should_exit() 
    And exit logic is delegated entirely to the PositionManager
    
  Scenario: No shared mutable state
    Given a ComposedStrategy with SignalGenerator S and PositionManager P
    Then S cannot read P's internal state
    And P cannot read S's internal state
    And they communicate ONLY through the defined interface:
      | From | To | Data |
      | S | Engine | Signal { direction, strength, metadata } |
      | Engine | P | Position { entry_price, entry_bar, ... } |
      | P | Engine | Action { hold, adjust_stop, scale, exit } |
```

### 1.3 The Trait Boundaries

Rather than code, here's the conceptual interface each component type exposes:

```gherkin
Feature: Component Interface Contracts
  Clean trait boundaries enforce separation of concerns.

  Scenario: SignalGenerator contract
    Given a SignalGenerator implementation
    Then it must implement:
      | Method | Input | Output |
      | name() | - | String identifier |
      | warmup_bars() | - | usize (minimum data needed) |
      | generate() | Bar, market_state | Option<Signal> |
      | parameter_spec() | - | Vec<ParameterDef> |
    And it must NOT:
      - Track position state
      - Know about current holdings
      - Make exit decisions

  Scenario: PositionManager contract
    Given a PositionManager implementation
    Then it must implement:
      | Method | Input | Output |
      | name() | - | String |
      | on_entry() | entry_bar, entry_price | initial internal state |
      | on_bar() | bar, position | Action enum |
      | stop_price() | - | Option<f64> |
      | parameter_spec() | - | Vec<ParameterDef> |
    And Action can be: Hold, AdjustStop(price), ScaleOut(pct), Exit

  Scenario: ExecutionModel contract
    Given an ExecutionModel implementation
    Then it must implement:
      | Method | Input | Output |
      | attempt_fill() | signal, current_bar, next_bar | FillResult |
      | gap_policy() | - | GapPolicy enum |
    And FillResult includes: filled (bool), fill_price, slippage

  Scenario: SignalFilter contract
    Given a SignalFilter implementation
    Then it must implement:
      | Method | Input | Output |
      | allow_signal() | signal, bar, market_state | bool |
      | force_exit() | position, bar, market_state | bool |
    And filters can suppress entries OR force exits (regime changes)
```

---

## Part 2: Data Architecture

### 2.1 Auto-Fetching Data Layer

One of v1's friction points was the Data Panel. v2 treats data as infrastructure, not UI:

```gherkin
Feature: Transparent Data Infrastructure
  Data fetching happens automatically and invisibly.

  Scenario: First launch experience
    Given TrendLab v2 is launched for the first time
    And no local cache exists
    Then a progress indicator shows "Initializing data universe..."
    And the 479-symbol universe is fetched from Yahoo Finance
    And data is stored in Parquet format at ~/.trendlab/data/
    And YOLO mode becomes available once warmup data (200 bars) is cached

  Scenario: Subsequent launch with stale data
    Given cache was last updated 3 days ago
    When TrendLab launches
    Then only the delta (3 days of bars per symbol) is fetched
    And appended to existing Parquet files
    And the UI is immediately usable during background sync

  Scenario: Data access is abstraction-hidden
    Given the backtest engine requests AAPL daily bars
    Then it calls: data_store.get_bars("AAPL", timeframe, date_range)
    And the DataStore handles: cache hit, cache miss, partial fetch
    And the backtest engine never knows where data came from

  Scenario: Configurable universe (power users only)
    Given a user wants to test on crypto or forex
    When they edit ~/.trendlab/config.toml
    Then they can specify: custom symbol lists, alternative data sources
    And the default remains the 479-stock trend-following universe
```

### 2.2 Data Schema for Composition

```gherkin
Feature: Data Structures Support Composition
  The data model is designed for component isolation.

  Scenario: Bar struct is component-agnostic
    Given a Bar struct
    Then it contains:
      | Field | Type | Purpose |
      | timestamp | DateTime | Bar identity |
      | open, high, low, close | f64 | OHLC data |
      | volume | f64 | Volume |
      | atr_14 | f64 | Pre-computed ATR(14) |
      | adx_14 | f64 | Pre-computed ADX(14) |
    And indicators are pre-computed in the data layer, not per-strategy

  Scenario: Position struct carries entry context
    Given a Position is created on entry
    Then it contains:
      | Field | Type | Purpose |
      | entry_bar_idx | usize | Which bar we entered |
      | entry_price | f64 | Actual fill price |
      | entry_signal | Signal | The signal that caused entry |
      | direction | Direction | Long or Short |
      | quantity | f64 | Position size |
      | high_since_entry | f64 | Running max (for trailing) |
      | low_since_entry | f64 | Running min (for shorts) |
    And high_since_entry/low_since_entry update automatically each bar
```

---

## Part 3: The Backtest Engine

### 3.1 Event-Driven Simulation

```gherkin
Feature: Clean Event Loop
  The backtest engine orchestrates components without coupling them.

  Scenario: Single-bar processing cycle
    Given the engine is processing bar N
    Then the cycle is:
      1. Update position tracking (high_since_entry, etc.)
      2. Ask PositionManager: "Any action on current position?"
         - If Exit or StopHit → record exit, clear position
         - If AdjustStop → update internal stop level
      3. If no position, ask SignalGenerator: "Any signal?"
         - If signal, ask SignalFilter: "Allow this signal?"
         - If allowed, ask ExecutionModel: "Can we fill?"
         - If filled → create Position, notify PositionManager
      4. Record equity, metrics

  Scenario: Position manager checks happen FIRST
    Given I am in a position
    And the current bar gaps down through my stop
    Then the PositionManager exit check happens before signal generation
    And I exit at the stop (or gap fill price per execution model)
    And I do NOT simultaneously enter a new position on the same bar
    
  Scenario: No lookahead bias
    Given the engine is processing bar N
    Then only data from bars 0..=N is visible
    And bar N+1 data is used ONLY for fill simulation (next bar open)
    And all indicators use only past data
```

### 3.2 Metrics and Attribution

```gherkin
Feature: Component-Level Attribution
  Track which component contributed to which outcome.

  Scenario: Exit reason tracking
    Given a trade exits
    Then the exit is tagged with reason:
      | Reason | Meaning |
      | SignalExit | Signal generator said exit (e.g., crossover reversed) |
      | StopHit | Position manager's stop was triggered |
      | FilterForceExit | Filter detected regime change, forced exit |
      | MaxHoldingPeriod | Position manager's time limit hit |
      | EndOfData | Backtest ended while in position |

  Scenario: Component statistics rollup
    Given YOLO has run 1000 iterations
    When I view ComponentStats
    Then I see:
      | Component | Median Sharpe | Win Rate | Avg Bars Held |
      | Signal: FiftyTwoWeekBreakout | 0.31 | 42% | 67 |
      | Signal: Supertrend | 0.28 | 45% | 43 |
      | PosMgr: ATRTrailing | 0.35 | 44% | 52 |
      | PosMgr: SinceEntryTrailing | 0.33 | 43% | 58 |
    And I can identify: "ATR Trailing is the best exit method"
    And I can identify: "52-Week Breakout WITH ATR Trailing = 0.38 Sharpe"
```

---

## Part 4: Monte Carlo Search

### 4.1 Structural Sampling

```gherkin
Feature: True Combinatorial Search
  YOLO explores structure, not just parameters.

  Scenario: Warmup phase explores uniformly
    Given YOLO starts with warmup_iterations = 200
    Then the first 200 iterations sample uniformly:
      - Each SignalGenerator type used ~18 times
      - Each PositionManager type used ~20 times
      - Random parameter values within defined ranges
    And no exploitation bias yet

  Scenario: Exploitation phase focuses on winners
    Given warmup is complete
    And leaderboard shows top performers
    When exploitation begins
    Then sampling is weighted toward:
      - Top 10% of structural combinations (by median cross-symbol Sharpe)
      - Parameter neighborhoods around best configs
    But 20% of samples remain uniform (exploration budget)

  Scenario: Component recombination discovers novel combinations
    Given warmup found:
      - FiftyTwoWeekBreakout + PercentTrailing = good
      - SuperTrend + ATRTrailing = good
    When structural sampling continues
    Then it naturally tests:
      - FiftyTwoWeekBreakout + ATRTrailing (novel)
      - SuperTrend + PercentTrailing (novel)
    And may discover that signal/exit pairings matter more than either alone
```

### 4.2 Robustness Scoring

```gherkin
Feature: Cross-Symbol Robustness
  Leaderboard ranks by consistency, not peak performance.

  Scenario: Robustness score calculation
    Given a strategy config has been tested on 50 symbols
    Then robustness score combines:
      | Factor | Weight | Purpose |
      | Median Sharpe | 40% | Central tendency |
      | Symbol hit rate | 30% | % of symbols with Sharpe > 0 |
      | Sharpe std dev | -20% | Penalize inconsistency |
      | Worst symbol Sharpe | 10% | Floor matters |

  Scenario: Lucky configs get filtered out
    Given config A: median Sharpe 0.45, hit rate 35%
    And config B: median Sharpe 0.32, hit rate 75%
    Then config B ranks higher (more robust)
    Because config A likely overfit to specific symbols

  Scenario: Walk-forward validation (optional)
    Given power user enables walk_forward_validation
    Then each config is tested with:
      - In-sample: first 80% of date range
      - Out-of-sample: last 20% of date range
    And robustness score includes OOS degradation factor
```

---

## Part 5: TUI Architecture

### 5.1 Preserved Strengths

```gherkin
Feature: Vim-Native Experience Preserved
  All navigation patterns from v1 carry forward.

  Scenario: Global keybindings
    Given I am anywhere in the application
    Then these keys always work:
      | Key | Action |
      | 1-4 | Jump to panel (reduced from 1-6) |
      | ? | Toggle Help overlay |
      | q | Quit |
      | Esc | Cancel/back |

  Scenario: List navigation
    Given I am in any panel with a list
    Then:
      | Key | Action |
      | j/↓ | Move down |
      | k/↑ | Move up |
      | gg | Jump to first |
      | G | Jump to last |
      | Ctrl+d | Page down |
      | Ctrl+u | Page up |

  Scenario: Value adjustment
    Given I am on a configurable value
    Then:
      | Key | Action |
      | h/← | Decrease |
      | l/→ | Increase |
      | H | Big decrease (10x step) |
      | L | Big increase (10x step) |
```

### 5.2 Simplified Panel Structure

```gherkin
Feature: Four-Panel Layout
  Reduced from six panels to four.

  Background:
    v1 had: Data, Strategy, Sweep, Results, Chart, Help
    v2 has: Home, Results, Chart, Help (overlay)

  Scenario: Home panel (replaces Data + Strategy + Sweep)
    Given I am in the Home panel
    Then I see:
      - Big "Start YOLO" button / [Enter] prompt
      - Current config summary (iterations, randomization %)
      - Data status (symbols cached, last update)
      - [c] to open Config modal

  Scenario: Results panel (enhanced)
    Given I am in the Results panel
    Then I can cycle views with [v]:
      | View | Shows |
      | Leaderboard | Top configs by robustness |
      | ComponentStats | Performance by signal/exit type |
      | RecentRuns | Last N iterations |
      | SymbolBreakdown | Per-symbol results for selected config |

  Scenario: Chart panel (preserved)
    Given I am in the Chart panel
    Then I see equity curve for selected result
    And can toggle: [m] chart mode, [d] drawdown overlay
    And can cycle symbols: [n] next, [p] previous

  Scenario: Help is an overlay, not a panel
    Given I press [?]
    Then a modal overlay appears with context-sensitive help
    And pressing [?] again or [Esc] dismisses it
    And help content matches current panel/mode
```

### 5.3 Config Modal (Power Users)

```gherkin
Feature: Optional Configuration
  Advanced settings available but not required.

  Scenario: Quick config access
    Given I am on the Home panel
    When I press [c]
    Then a modal appears with grouped settings:
      | Group | Settings |
      | Search | iterations, randomization_pct, warmup_pct |
      | Components | signal_pool, exit_pool, execution_pool, filter_pool |
      | Validation | walk_forward, min_trades, min_symbols |
      | Export | auto_pine, artifact_dir |

  Scenario: Component pool toggles
    Given I am in the Components section of config
    Then I see checkboxes for each component:
      [x] FiftyTwoWeekBreakout
      [x] Supertrend
      [ ] ParabolicSar (disabled)
      ...
    And I can enable/disable any component
    And disabled components are excluded from YOLO sampling

  Scenario: Smart defaults
    Given I launch TrendLab fresh
    Then all config values have sensible defaults:
      | Setting | Default |
      | iterations | 500 |
      | randomization_pct | 50% |
      | warmup_pct | 40% |
      | all signals | enabled |
      | all exits | enabled |
```

---

## Part 6: Export and Integration

### 6.1 Pine Script Generation

```gherkin
Feature: Strategy Export to TradingView
  One-key export preserves parity with backtest results.

  Scenario: Export from leaderboard
    Given I select a config in the Results panel
    When I press [P]
    Then:
      1. StrategyArtifact JSON saved to artifacts/exports/
      2. Pine Script v6 generated in pine-scripts/strategies/
      3. Test vectors included for parity validation
      4. Success message shows file path

  Scenario: Pine Script structure
    Given an exported Pine Script
    Then it contains:
      | Section | Purpose |
      | //@version=6 header | TradingView compatibility |
      | Indicator calculations | Match backtest indicators exactly |
      | Entry conditions | Signal generator logic |
      | Exit conditions | Position manager logic |
      | Strategy calls | strategy.entry, strategy.exit |
      | // PARITY_TEST comments | Expected values for validation |

  Scenario: Parity validation
    Given I load the Pine Script in TradingView
    And I add the same symbol/timeframe as the backtest
    Then the PARITY_TEST comments show expected values
    And I can verify: entry dates, exit dates, P&L match within tolerance
```

### 6.2 Artifact Persistence

```gherkin
Feature: Research Artifacts
  All research is automatically persisted and queryable.

  Scenario: Session persistence
    Given a YOLO session runs 500 iterations
    Then automatically saved:
      | Artifact | Location |
      | Session config | artifacts/sessions/{timestamp}/config.json |
      | Iteration logs | artifacts/sessions/{timestamp}/iterations.parquet |
      | Leaderboard snapshot | artifacts/sessions/{timestamp}/leaderboard.json |
      | Component stats | artifacts/sessions/{timestamp}/component_stats.json |

  Scenario: Resume capability
    Given a previous session exists
    When I launch TrendLab
    And I press [r] for resume
    Then I see list of previous sessions
    And I can select one to resume
    And YOLO continues from where it left off
```

---

## Part 7: Error Handling and Observability

### 7.1 Graceful Degradation

```gherkin
Feature: Robust Error Handling
  Failures in one component don't crash the system.

  Scenario: Bad data for one symbol
    Given symbol XYZ has corrupt data
    When YOLO iteration tests XYZ
    Then the symbol is skipped for this iteration
    And a warning is logged: "Skipping XYZ: insufficient data"
    And the iteration continues with other symbols
    And XYZ is flagged for data refresh

  Scenario: Component throws exception
    Given a buggy custom SignalGenerator panics
    Then the iteration catches the panic
    And logs: "SignalGenerator 'Custom' panicked on AAPL bar 1234"
    And the iteration is recorded as failed
    And YOLO continues to next iteration
```

### 7.2 Logging and Visibility

```gherkin
Feature: Research Visibility
  See what's happening in real-time.

  Scenario: Status bar information
    Given YOLO is running
    Then status bar shows:
      | Field | Example |
      | Iteration | 234 / 500 |
      | Rate | 12.3 iter/sec |
      | Best Sharpe | 0.42 |
      | Current | testing Supertrend + Chandelier on MSFT |

  Scenario: Log panel events
    Given YOLO is running
    Then log panel shows significant events:
      - "New leaderboard entry: rank #3, Sharpe 0.38"
      - "Component winner: ATRTrailing median Sharpe 0.35"
      - "Data fetch: updated 15 symbols"
    And errors show in red with context
```

---

## Part 8: Implementation Priorities

### 8.1 Phase 1: Foundation (Core Value)

```gherkin
Feature: MVP Deliverable
  The minimum system that solves the stickiness problem.

  Must have:
    - [ ] Component traits (SignalGenerator, PositionManager, ExecutionModel, Filter)
    - [ ] 3 SignalGenerators: FiftyTwoWeekBreakout, Supertrend, MACrossover
    - [ ] 3 PositionManagers: ATRTrailing, SinceEntryTrailing, FixedStop
    - [ ] 1 ExecutionModel: NextBarOpen
    - [ ] 1 Filter: None
    - [ ] Backtest engine with state isolation
    - [ ] Basic TUI: Home + Results panels
    - [ ] YOLO mode with uniform sampling
    - [ ] Cross-symbol leaderboard

  Validation:
    When I run YOLO for 100 iterations
    Then FiftyTwoWeekBreakout + ATRTrailing performs comparably to Supertrend
    Because they now have equivalent position management
```

### 8.2 Phase 2: Full Component Library

```gherkin
Feature: Complete Strategy Universe
  All 1,760 structural combinations available.

  Add:
    - [ ] Remaining 8 SignalGenerators
    - [ ] Remaining 7 PositionManagers
    - [ ] Remaining 3 ExecutionModels
    - [ ] Remaining 3 Filters
    - [ ] Component statistics view
    - [ ] Parameter range exploration
```

### 8.3 Phase 3: Polish and Export

```gherkin
Feature: Production Ready
  Full feature parity with v1 vision, minus the friction.

  Add:
    - [ ] Pine Script export
    - [ ] Walk-forward validation
    - [ ] Session persistence and resume
    - [ ] Chart panel with full visualization
    - [ ] Config modal for power users
```

---

## Summary: What's Different

| Aspect | TrendLab v1 | TrendLab v2 |
|--------|------------|-------------|
| **Architecture** | Monolithic strategies | Composable components |
| **Stickiness** | Signal/exit coupled | Strict state isolation |
| **Data** | Manual panel | Auto-fetch background |
| **Modes** | Sweep + YOLO | YOLO only |
| **Panels** | 6 panels | 4 panels (Help = overlay) |
| **Config** | Required upfront | Optional modal |
| **Startup** | Configure → Fetch → Run | Press Enter → Run |
| **Attribution** | Strategy-level | Component-level |

The core fix: **position managers track state SINCE ENTRY, never using the signal generator's references.** This single architectural decision eliminates the stickiness problem and enables true combinatorial search.

---
