# TrendLab v2: Implementation Backlog

> Phases 2-6 for future expansion. See [detailed-plan.md](detailed-plan.md) for the active Phase 1 work.

---

## Table of Contents

1. [Phase 2: Data Infrastructure (trendlab-data)](#phase-2-data-infrastructure)
2. [Phase 3: Engine Spine (trendlab-core)](#phase-3-engine-spine)
3. [Phase 4: YOLO Discovery (trendlab-yolo)](#phase-4-yolo-discovery)
5. [Phase 5: Terminal UI (trendlab-tui)](#phase-5-terminal-ui)
6. [Phase 6: Export & Pine Parity (trendlab-export)](#phase-6-export--pine-parity)

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

### 2.2 Key Components

- **YahooClient**: Fetch OHLCV data with rate limiting
- **ParquetCache**: ZSTD-compressed caching with delta sync
- **Universe**: Symbol universe management (US equities, futures, custom)
- **Indicators**: Pre-computed ATR, ADX using Polars lazy operations

### 2.3 BDD Scenarios

```gherkin
Feature: Data Caching
  Scenario: First fetch caches to parquet
  Scenario: Subsequent reads use cache
  Scenario: Delta sync fetches only new data

Feature: Lazy Data Loading
  Scenario: Data is scanned lazily
  Scenario: Filtering happens before loading
```

### 2.4 Implementation Order

1. error.rs - Data-specific errors
2. client/yahoo.rs - Yahoo Finance client
3. cache/parquet.rs - Parquet caching (HashSet for date dedup)
4. universe.rs - Symbol universe
5. indicators.rs - ATR, ADX computation
6. lib.rs - Expose unified API
7. Integration tests + BDD scenarios

---

## Phase 3: Engine Spine

### 3.1 Key Components

- **BacktestEngine**: The sacred simulation loop
- **DonchianBreakout**: First SignalGenerator
- **AtrTrailingStop**: First PositionManager (SinceEntryTrailingExtreme mode)
- **NextOpenFill**: First ExecutionModel

### 3.2 The Event Loop (per bar N)

1. Update position tracking (high/low since entry)
2. Check EXITS before entries (stop via EM → PM action → filter force exit)
3. Check entries if no position (signal → filter → fill at N+1)
4. Record equity snapshot

### 3.3 BDD Scenarios

```gherkin
Feature: Trade Lifecycle
  Scenario: Signal generates fill at next open
  Scenario: Exit checks occur before entry checks
  Scenario: Stop hit fills correctly
  Scenario: Gap through stop fills at open

Feature: Determinism
  Scenario: Same config produces same results
  Scenario: Fresh PM state per run
```

### 3.4 Implementation Order

1. engine.rs - Core backtest loop
2. signals/donchian.rs - First SignalGenerator
3. position_managers/atr_trailing.rs - First PositionManager
4. execution/next_open.rs - First ExecutionModel
5. BDD step definitions + Determinism tests

---

## Phase 4: YOLO Discovery

### 4.1 Key Components

- **Genome**: Encodes composition + parameters
- **RobustnessScorer**: Median Sharpe, hit rate, consistency, floor with penalties
- **LeaderboardSet**: Three disentangled leaderboards (Invariant E)
- **StructuralSampler**: Monte Carlo that samples structure first

### 4.2 Robustness Formula

```
base = w_median * median_sharpe + w_hit * hit_rate + w_consistency * -std + w_floor * floor
robustness = base * (1 - dd_penalty * max_dd) * (1 - fragility * cost_sens)
```

### 4.3 BDD Scenarios

```gherkin
Feature: YOLO Sampling
  Scenario: Warmup samples uniformly
  Scenario: Exploitation weights by robustness
  Scenario: Structural mutation swaps one component

Feature: Three Leaderboards
  Scenario: Signal Quality leaderboard isolates signals
  Scenario: Three leaderboards before trusting overall
```

### 4.4 Implementation Order

1. genome.rs - Genome representation
2. registry.rs - Component registry
3. robustness.rs - Scoring formulas
4. leaderboard.rs - Three leaderboards
5. sampler.rs - Structural Monte Carlo
6. session.rs - YOLO session state machine
7. attribution.rs - Component attribution

---

## Phase 5: Terminal UI

### 5.1 Key Components

- **App**: Central state (panels render, don't own)
- **PanelId**: Home (1), Results (2), Chart (3)
- **Keybindings**: Vim-native (j/k/h/l, gg/G, 1-3, Enter, P, ?)
- **Colors**: Semantic system (green=positive, red=negative, etc.)

### 5.2 Panel Layout

```
┌─Home─────────────┬─Results──────────────────────┬─Chart─────────────┐
│ Universe: US30   │ Rank │ Signal    │ PM      │ │ [Equity Curve]    │
│ YOLO: ● Running  │ 1    │ Donchian  │ ATRTr.  │ │ [Trade Markers]   │
│ [Enter] to start │ ...  │           │         │ │ Sharpe: 0.45      │
└──────────────────┴──────────────────────────────┴───────────────────┘
```

### 5.3 Implementation Order

1. app.rs - Central state
2. event.rs - Event handling
3. keybinds.rs - Vim keybindings
4. colors.rs - Semantic color system
5. panels/home.rs, results.rs, chart.rs
6. modals/help.rs
7. main.rs - Render loop

---

## Phase 6: Export & Pine Parity

### 6.1 Key Components

- **StrategyArtifact**: JSON matching `schemas/strategy-artifact.schema.json`
- **PineGenerator**: Pine Script v6 code generation
- **ParityVectors**: Entry/exit dates + prices for validation

### 6.2 Parity Tolerances

- Entry dates: within 1 bar
- Exit prices: within 0.1%
- Total return: within 1%

### 6.3 BDD Scenarios

```gherkin
Feature: Pine Parity
  Scenario: Entry dates match
  Scenario: Exit prices match
  Scenario: Total return matches
```

### 6.4 Implementation Order

1. artifact.rs - StrategyArtifact struct
2. pine.rs - Pine Script generation
3. parity.rs - Parity test vectors
4. export.rs - Export orchestration

---

## Verification Checklist

| Phase | Checkpoint |
|-------|------------|
| 2 | Yahoo fetch works, Parquet ZSTD, LazyFrame scans |
| 3 | Full backtest produces trades, determinism verified |
| 4 | YOLO samples structures, three leaderboards populated |
| 5 | TUI renders, vim keys work, Enter starts YOLO |
| 6 | Valid JSON export, Pine compiles, parity within tolerance |

---

*Last updated: 2026-01-21*
