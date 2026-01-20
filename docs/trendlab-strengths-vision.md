# TrendLab: Strengths & Streamlined Vision

## Executive Summary

TrendLab is a research-grade trend-following backtesting lab built in Rust with a terminal UI. This document captures what makes TrendLab exceptional and envisions a streamlined "YOLO-only" version that removes friction while preserving the powerful discovery engine.

---

## Part 1: What TrendLab Does Well

### 1.1 Vim-Style Navigation

TrendLab embraces terminal-native efficiency with consistent vim keybindings throughout:

```gherkin
Feature: Vim-Style Navigation
  The application provides muscle-memory-friendly navigation
  using standard vim keybindings across all panels.

  Scenario: Navigate lists with j/k
    Given I am in any panel with a list
    When I press "j"
    Then the selection moves down one item
    When I press "k"
    Then the selection moves up one item

  Scenario: Adjust values with h/l
    Given I am on a numeric parameter field
    When I press "l" or right arrow
    Then the value increases by one step
    When I press "h" or left arrow
    Then the value decreases by one step

  Scenario: Jump navigation with gg and G
    Given I am in the Results panel with many entries
    When I press "gg"
    Then the selection jumps to the first item
    When I press "G"
    Then the selection jumps to the last item

  Scenario: Page navigation with Ctrl+d/u
    Given I am viewing a long list
    When I press "Ctrl+d"
    Then the view scrolls down half a page
    When I press "Ctrl+u"
    Then the view scrolls up half a page

  Scenario: Direct panel access with number keys
    Given I am anywhere in the application
    When I press "1"
    Then I jump directly to the Data panel
    When I press "2"
    Then I jump directly to the Strategy panel
    When I press "5"
    Then I jump directly to the Chart panel
    When I press "?"
    Then I jump directly to the Help panel
```

### 1.2 Color System

The TUI uses a carefully designed color palette that conveys meaning at a glance:

```gherkin
Feature: Semantic Color System
  Colors provide instant visual feedback about state and meaning.

  Scenario: Active vs inactive panels
    Given multiple panels are visible
    When I focus on a panel
    Then that panel's border turns bright blue
    And unfocused panels have dim gray borders

  Scenario: Selection highlighting
    Given I am navigating a list
    Then the selected item has a cyan highlight bar
    And selected checkboxes show green check marks

  Scenario: Data status indicators
    Given I am viewing the ticker list
    When a ticker has cached data
    Then it displays a green dot indicator
    When a ticker needs data fetch
    Then no indicator appears

  Scenario: Metric coloring
    Given I am viewing backtest results
    When Sharpe ratio is above 0.3
    Then it displays in green
    When max drawdown exceeds 30%
    Then it displays in red
    And neutral values display in default foreground

  Scenario: Help panel key highlighting
    Given I am in the Help panel
    Then keyboard shortcuts display in green
    And section headers display in magenta
    And descriptions display in standard foreground
```

### 1.3 Context-Sensitive Help System

The Help panel (Tab 6 / "?") provides comprehensive, searchable documentation:

```gherkin
Feature: Interactive Help System
  In-app documentation with sections, search, and scroll.

  Scenario: Section navigation
    Given I am in the Help panel
    Then I see section tabs: Global, Data, Strategy, Sweep, Results, Chart, Features
    When I press "l" or right arrow
    Then I move to the next section
    When I press "h" or left arrow
    Then I move to the previous section

  Scenario: Scrollable detailed content
    Given I am viewing a help section with long content
    When I press "j" or down arrow
    Then the content scrolls down
    And the scroll position indicator updates

  Scenario: Search within help
    Given I am in the Help panel
    When I press "/" to enter search mode
    And I type "YOLO"
    Then matching text is highlighted in yellow
    And the match count displays
    When I press "n"
    Then I jump to the next match
    When I press "N"
    Then I jump to the previous match

  Scenario: Quick reference format
    Given I view any help section
    Then I see a "Quick Reference" block with key-description pairs
    And I see a "Details" block with expanded explanations
```

### 1.4 Leaderboard System

The cross-symbol leaderboard ranks strategy configurations by robust performance:

```gherkin
Feature: Cross-Symbol Leaderboard
  Aggregates performance across all tested symbols to find robust configs.

  Scenario: Leaderboard ranking
    Given YOLO mode has completed multiple iterations
    Then the leaderboard shows top configurations
    And each entry displays: strategy name, config params, median Sharpe, hit rate
    And entries are ranked by robustness score

  Scenario: View leaderboard details
    Given I am in Results panel with Leaderboard view
    When I select an entry with j/k
    And I press Enter
    Then the Chart panel displays the combined equity curve
    And the chart title shows the strategy name and config

  Scenario: Component statistics view
    Given I am in Results panel
    When I press "v" to cycle views
    And I reach "ComponentStats" view
    Then I see performance breakdown by signal type
    And I see performance breakdown by position manager
    And I see performance breakdown by filter type
```

### 1.5 Real-Time Logging

The status bar and log output provide continuous feedback:

```gherkin
Feature: Progress Visibility
  Real-time feedback during long-running operations.

  Scenario: YOLO iteration progress
    Given YOLO mode is running
    Then the status bar shows current iteration number
    And the status bar shows elapsed time
    And the status bar shows iterations per second
    And the status bar shows current strategy being tested

  Scenario: Log messages
    Given any operation is in progress
    Then significant events appear in the log area
    Including: data fetches, sweep starts, new leaderboard entries
    And errors display in red with context

  Scenario: Background task indicators
    Given data is being fetched
    Then a spinner or progress indicator is visible
    And the fetch status shows symbols remaining
```

### 1.6 Pine Script Export

One-button export to TradingView Pine Script v6:

```gherkin
Feature: Pine Script Generation
  Export winning configurations to TradingView-compatible Pine Script.

  Scenario: Export from leaderboard
    Given I am in Results panel with Leaderboard view
    And I have selected a configuration
    When I press "P" (capital P)
    Then a StrategyArtifact JSON is created in artifacts/exports/
    And a Pine Script is generated in pine-scripts/strategies/
    And a success message displays the output path

  Scenario: Artifact content
    Given a StrategyArtifact has been exported
    Then it contains: strategy_id, timeframe, fill_model
    And it contains: indicator definitions with parameters
    And it contains: entry/exit rules in Pine-friendly DSL
    And it contains: parity test vectors for validation
```

---

## Part 2: Trend-Following Strategy Components

### 2.1 Signal Generators (Entry Logic)

TrendLab tests 11 distinct signal generation methods:

```gherkin
Feature: Signal Generator Pool
  Diverse entry signal methods for trend detection.

  Signal Types:
    | Type                | Description                              |
    | FiftyTwoWeekBreakout| Price breaks N-period high               |
    | DonchianBreakout    | Channel breakout with separate exit      |
    | BollingerBreakout   | Price exceeds Bollinger Band upper       |
    | KeltnerBreakout     | Price exceeds Keltner Channel upper      |
    | Supertrend          | ATR-based trend direction flip           |
    | ParabolicSar        | Parabolic stop-and-reverse               |
    | MaCrossover         | Fast MA crosses above slow MA            |
    | Momentum            | Price > Price[N] (time-series momentum)  |
    | RocMomentum         | Rate of change exceeds threshold         |
    | AroonCrossover      | Aroon Up crosses above Aroon Down        |
    | TrendFlip           | Trend direction change detection         |

  Scenario: Parameter ranges for exploration
    Given the signal pool includes 52-Week Breakout
    Then lookback ranges from 50 to 252 days
    And entry_pct ranges from 0.90 to 1.00
    And max_age ranges from 1 to 5 bars
```

### 2.2 Position Managers (Exit Logic)

10 distinct exit/stop methods to decouple entry from exit:

```gherkin
Feature: Position Manager Pool
  Sophisticated exit logic independent of entry signals.

  Position Manager Types:
    | Type                  | Description                               |
    | AtrTrailingStop       | Trail by N * ATR from recent high        |
    | ChandelierExit        | ATR-based stop from highest high         |
    | PercentTrailing       | Trail by fixed percentage from high      |
    | SinceEntryTrailingHigh| Trail from max high since entry          |
    | FrozenReferenceExit   | Exit based on reference locked at entry  |
    | TimeDecayStop         | Stop tightens over holding period        |
    | MaxHoldingPeriod      | Force exit after N bars                  |
    | FixedStop             | Fixed percentage stop-loss               |
    | BreakevenThenTrail    | Move to breakeven, then trail            |
    | SignalBasedExit       | Exit when entry signal reverses          |

  Scenario: ATR trailing stop behavior
    Given I enter a long position at $100
    And ATR is $2 with multiplier 3.0
    Then initial stop is at $94 (100 - 3*2)
    When price rises to $110 with ATR still $2
    Then stop ratchets up to $104 (110 - 3*2)
    And stop never moves down (trailing only)
```

### 2.3 Execution Models

4 order types determine how signals translate to fills:

```gherkin
Feature: Execution Model Pool
  Different order types for realistic fill simulation.

  Execution Types:
    | Type           | Description                                |
    | NextBarOpen    | Signal on close, fill at next bar open    |
    | CloseOnSignal  | Fill at the close when signal fires       |
    | StopOrder      | Fill when price crosses trigger level     |
    | LimitOrder     | Fill at better price or not at all        |

  Scenario: Stop order execution
    Given a breakout signal triggers at $100 level
    And gap policy is "fill_at_open"
    When next bar opens at $102 (gaps above)
    Then fill price is $102 (the open)
    When next bar opens at $99, high reaches $101
    Then fill price is $100 (the stop level)
```

### 2.4 Signal Filters

4 regime filters to suppress signals in unfavorable conditions:

```gherkin
Feature: Filter Pool
  Regime-aware signal suppression.

  Filter Types:
    | Type       | Description                                    |
    | None       | No filtering (baseline)                        |
    | Adx        | Only trade when ADX > threshold (trending)     |
    | MaRegime   | Only trade when price above/below MA           |
    | Volatility | Only trade within ATR percentile bounds        |

  Scenario: ADX filter behavior
    Given ADX filter is active with threshold 25
    When ADX reads 30
    Then entry signals are allowed
    When ADX reads 18
    Then entry signals are suppressed
    And "filter blocked" is logged
```

---

## Part 3: The True Monte Carlo Vision

### 3.1 The Stickiness Problem (What We Fixed)

```gherkin
Feature: Stickiness Problem Resolution
  Strategies no longer have unfair structural advantages.

  Background:
    The "stickiness problem" was that strategies like 52-Week High
    used the SAME rolling reference for both entry AND exit.
    As trends worked, the exit threshold kept moving away,
    making exits harder the better the trend performed.

  Scenario: Old (sticky) behavior
    Given 52-Week High strategy with rolling reference
    When price breaks out at $100 (new high)
    And price trends to $110 (new rolling high)
    Then exit threshold also rises to $99 (0.9 * 110)
    And a 10% pullback from $110 to $99 triggers exit
    But a 5% pullback to $104.50 does NOT exit
    Result: Strategy holds too long in winners

  Scenario: New (fixed) behavior
    Given 52-Week High with Since-Entry Trailing High
    When price breaks out at $100
    Then exit reference is max high SINCE ENTRY
    When price trends to $110
    Then exit threshold is $99 (0.9 * 110)
    And threshold only rises, never falls
    Result: Fair comparison with Supertrend
```

### 3.2 Structural Monte Carlo

The true universe of possibilities is combinatorial:

```gherkin
Feature: Structural Monte Carlo
  YOLO explores STRUCTURE, not just parameters.

  Search Space:
    TRUE_UNIVERSE = SignalTypes (11)
                  × PositionManagers (10)
                  × ExecutionModels (4)
                  × Filters (4)
                  × Parameters (continuous)

    = 1,760 structural combinations
    × infinite parameter space

  Scenario: Structural sampling
    Given YOLO mode is in Structural mode
    And randomization_pct is 50%
    Then each iteration may:
      - Swap the signal generator (30% chance)
      - Swap the position manager (30% chance)
      - Swap the execution model (20% chance)
      - Add/remove filters (20% chance)
    And parameter values jitter within their ranges

  Scenario: Component recombination
    Given iteration discovers "52-Week Breakout + ATR Trailing"
    When next iteration samples
    Then it might test "52-Week Breakout + Chandelier Exit"
    Or "Donchian Breakout + ATR Trailing"
    Result: Find which COMBINATIONS work, not just which strategies
```

### 3.3 Separate Leaderboards

Isolate what's actually being tested:

```gherkin
Feature: Component-Level Analysis
  Identify which component contributes to performance.

  Scenario: Signal quality isolation
    Given all signals paired with same position manager (ATR Trail)
    Then leaderboard ranks SIGNAL QUALITY
    And we learn: "Which entry timing is best?"

  Scenario: Position manager isolation
    Given all tests use same entry signal (50-day breakout)
    Then leaderboard ranks POSITION MANAGEMENT
    And we learn: "Which stop/trail method is best?"

  Scenario: Component stats view
    Given YOLO has run 500+ iterations
    When I view ComponentStats in Results panel
    Then I see median Sharpe by signal type
    And I see median Sharpe by position manager
    And I see which combinations outperform
```

---

## Part 4: Vision for Streamlined YOLO-Only App

### 4.1 What to Remove

```gherkin
Feature: Simplified Interface
  Remove friction, keep power.

  Remove:
    - Data Panel: Auto-fetch on startup
    - Sweep Panel: YOLO is the only mode
    - Manual configuration complexity
    - Strategy panel detail editing

  Keep:
    - Chart Panel: Visualize results
    - Results Panel: Browse leaderboard
    - Help Panel: Documentation
    - Status bar: Progress visibility
    - Logging: Event feedback
```

### 4.2 Streamlined Data Flow

```gherkin
Feature: Auto-Data Universe
  No manual ticker selection required.

  Scenario: Startup data fetch
    Given the application launches
    Then it automatically loads the 479-symbol universe
    And fetches 30 years of daily data (1996-2026)
    And data is cached in Parquet format
    And missing data is fetched from Yahoo Finance

  Scenario: Incremental updates
    Given data was last fetched yesterday
    When application launches
    Then only fetch today's bars for each symbol
    And append to existing Parquet files
```

### 4.3 One-Button Research

```gherkin
Feature: YOLO-First Experience
  Launch → Scan → Results → Export

  Scenario: New user experience
    Given I launch TrendLab
    Then I see a simple prompt: "Run trend-following research? [Y/n]"
    When I press Enter or "Y"
    Then structural YOLO mode begins
    And data auto-fetches in background
    And results appear in real-time leaderboard

  Scenario: Streamlined configuration
    Given I want to customize before running
    When I press "c" for config
    Then I see:
      - Randomization %: [slider 0-100]
      - Max iterations: [number input]
      - Backtest period: [date range]
    And all other settings use smart defaults

  Scenario: Results to Pine in 3 keys
    Given YOLO has found winning configurations
    When I press "5" (Results panel)
    And I press "j" to select top config
    And I press "P" to export Pine
    Then Pine Script is generated and saved
    And path is displayed for copy to TradingView
```

### 4.4 Preserved Power Features

```gherkin
Feature: Power User Access
  Complexity available but not required.

  Scenario: Advanced configuration
    Given I press "Y" for YOLO config modal
    Then I can adjust:
      - Component pool weights
      - Parameter ranges
      - Filter combinations
      - Execution model distribution
      - Walk-forward settings
      - Robustness thresholds

  Scenario: Full leaderboard interaction
    Given YOLO has produced results
    Then I can:
      - Sort by any metric (s key)
      - Cycle view modes (v key)
      - View component breakdown
      - Export any config to Pine
      - View detailed equity curves

  Scenario: Vim navigation preserved
    Given I am anywhere in the streamlined app
    Then all vim keybindings work identically
    And j/k/h/l navigation is consistent
    And gg/G jump navigation works
    And number keys switch panels
```

---

## Part 5: Summary Tables

### Keyboard Reference (Streamlined App)

| Key | Context | Action |
|-----|---------|--------|
| `Enter` | Startup | Start YOLO research |
| `c` | Startup | Open config |
| `1-5` | Global | Jump to panel |
| `?` | Global | Open Help |
| `q` | Global | Quit |
| `j/k` | Lists | Navigate up/down |
| `h/l` | Values | Decrease/increase |
| `gg/G` | Lists | Jump to top/bottom |
| `Enter` | Results | View in Chart |
| `v` | Results | Cycle view mode |
| `s` | Results | Cycle sort column |
| `P` | Results | Export Pine Script |
| `m` | Chart | Cycle chart mode |
| `d` | Chart | Toggle drawdown |

### Strategy Components Summary

| Category | Count | Examples |
|----------|-------|----------|
| Signal Generators | 11 | 52-Week Breakout, Supertrend, MA Cross, Momentum |
| Position Managers | 10 | ATR Trail, Chandelier, Time Decay, Breakeven+Trail |
| Execution Models | 4 | Next Open, Close on Signal, Stop Order, Limit |
| Filters | 4 | None, ADX, MA Regime, Volatility |
| **Total Structural Combos** | **1,760** | Plus continuous parameter space |

### What Makes YOLO Special

| Feature | Benefit |
|---------|---------|
| Structural sampling | Tests COMBINATIONS, not just parameters |
| Component pools | Fair comparison across strategy families |
| Cross-symbol leaderboard | Finds robust configs, not lucky ones |
| Auto artifact export | Pine Script generation built-in |
| Warmup + exploitation | Statistical validity before winner focus |
| Real-time visibility | Watch discovery happen |

---

## Conclusion

TrendLab's strengths lie in:
1. **Vim-native efficiency** - Muscle memory navigation
2. **Visual clarity** - Semantic colors and clear hierarchy
3. **Comprehensive help** - Searchable, scrollable documentation
4. **Structural Monte Carlo** - Test the true universe of possibilities
5. **Robust ranking** - Cross-symbol validation eliminates luck
6. **Pine export** - Research to production in one keypress

The streamlined vision preserves all this power while removing friction:
- Auto-data fetching (no Data panel)
- YOLO-first (no Sweep panel complexity)
- One-button research flow
- Same powerful results and export

The goal: **Launch → Research → Export in under a minute, with 30 years of data across 479 symbols, testing 1,760+ structural combinations.**
