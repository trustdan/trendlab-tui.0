# TrendLab v2: Overview Plan

> A research-grade, terminal-native trend-following lab in Rust that explores a true strategy universe via structural Monte Carlo (YOLO) and surfaces robust winners.

---

## 1. Executive Summary

### Mission
Build a backtesting platform where **any signal generator** pairs with **any position manager** and **any execution model**—enabling fair comparison and surfacing strategies that are robust, not just lucky.

### The v1 Failure: Stickiness
TrendLab v1 bundled signal + exits + execution into monolithic "strategies." This created **stickiness**: exits using the same rolling reference as entries made positions impossible to exit (the stop "ran away" with new highs).

### The v2 Cure
- **Composition over bundles**: Strategy = SignalGenerator + PositionManager + ExecutionModel + SignalFilter
- **Explicit exit semantics**: Every extreme-based exit declares its reference mode
- **Disentangled leaderboards**: Know which *component* is winning, not just which bundle

### Product Stance: YOLO-First
- Launch → Press Enter → Structural YOLO begins
- No configuration required to see first results
- Live leaderboard updates as exploration runs
- **If the user must configure data before seeing results, UX failed.**

---

## 2. Non-Negotiable Invariants

These are architecture-level constraints. Violating them makes results untrustworthy.

| ID | Invariant | Why It Matters |
|----|-----------|----------------|
| **A** | Strategy = composition of independent components | Fair comparison requires isolation |
| **B** | Signal → Order → Fill separation | Prevents lookahead bias and entanglement |
| **C** | Exit reference semantics are explicit | Prevents stickiness (the v1 failure) |
| **D** | Execution is a first-class variable | Robustness testing requires varying fill assumptions |
| **E** | Multiple disentangled leaderboards | Know which *component* drives performance |
| **F** | Determinism & provenance | Every result must be reproducible |

### Exit Reference Modes (Invariant C)

Any extreme-based exit MUST declare one:

| Mode | Behavior |
|------|----------|
| `EntryFrozenReference` | Reference fixed at entry, never updates |
| `SinceEntryTrailingExtreme` | Tracks extreme since entry, not globally |
| `SeparateEntryExitLookbacks` | Entry uses one window, exit uses another |

**Forbidden**: Using the same rolling reference for entry AND exit.

---

## 3. Component Architecture

### Four Orthogonal Layers

```
┌─────────────────────────────────────────────────────────────┐
│                     Strategy (Composition)                   │
├─────────────────┬─────────────────┬─────────────────────────┤
│ SignalGenerator │ PositionManager │ ExecutionModel │ Filter │
│ "Should I       │ "How do I       │ "How do orders │ "Allow │
│  enter?"        │  manage this?"  │  become fills?"│ this?" │
└─────────────────┴─────────────────┴─────────────────────────┘
```

### Component Counts (Planned)

| Layer | Count | Examples |
|-------|-------|----------|
| SignalGenerator | 11 | Donchian breakout, MA crossover, TSMOM, ATH breakout |
| PositionManager | 10 | Fixed stop, ATR trailing, Chandelier, time-based |
| ExecutionModel | 4 | Next open, close, stop order, limit order |
| SignalFilter | 4 | ADX trend filter, volatility regime, seasonal |

**Structural space**: 11 × 10 × 4 × 4 = **1,760 combinations** before parameter variation.

### Contract Summary

| Component | MUST | MUST NOT |
|-----------|------|----------|
| SignalGenerator | Produce entry intents only | Track position state, make exit decisions |
| PositionManager | Initialize state at entry, declare exit mode | Access SignalGenerator internals |
| ExecutionModel | Declare fill timing, gaps, slippage | Peek at future bars |
| SignalFilter | Return allow/suppress decision | Modify signals or positions |

---

## 4. Crate Dependency Graph

```
trendlab-tui (binary: trendlab)
├── trendlab-yolo
│   ├── trendlab-core
│   └── trendlab-data
│       └── trendlab-core
└── trendlab-export
    └── trendlab-core
```

### Crate Responsibilities

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| **trendlab-core** | Traits, types, metrics, engine spine | chrono, serde, thiserror |
| **trendlab-data** | Yahoo fetch, Parquet cache, universe | polars, reqwest, tokio |
| **trendlab-yolo** | Structural sampling, ranking, attribution | trendlab-core, trendlab-data |
| **trendlab-tui** | Ratatui UI, vim keybindings, panels | ratatui, crossterm, tokio |
| **trendlab-export** | Pine Script, artifacts, repro bundles | trendlab-core, serde_json |

---

## 5. Implementation Phases

### Phase 1: Foundation (trendlab-core)

**Goal**: Define the type system and trait contracts that enforce invariants.

- [ ] Core types: `Bar`, `Signal`, `Order`, `Fill`, `Position`, `Trade`
- [ ] Traits: `SignalGenerator`, `PositionManager`, `ExecutionModel`, `SignalFilter`
- [ ] Exit reference mode enum
- [ ] Metrics calculation (Sharpe, Sortino, drawdown, CAGR)
- [ ] Property tests: state isolation, no lookahead

**Deliverable**: `cargo test -p trendlab-core` passes with invariant tests.

### Phase 2: Data Infrastructure (trendlab-data)

**Goal**: Invisible data layer that just works.

- [ ] Yahoo Finance OHLCV fetching
- [ ] Parquet caching with ZSTD compression
- [ ] Incremental delta sync (only fetch new bars)
- [ ] Symbol universe management
- [ ] Pre-computed indicators (ATR, ADX)
- [ ] Lazy Polars scans (never eager reads)

**Deliverable**: `fetch_bars("AAPL")` returns LazyFrame from cache or fetches.

### Phase 3: Engine Spine (trendlab-core)

**Goal**: The simulation loop that respects Signal → Order → Fill.

- [ ] Backtest engine with bar-by-bar processing
- [ ] Exit checks BEFORE entry checks per bar
- [ ] First SignalGenerator: Donchian breakout
- [ ] First PositionManager: ATR trailing stop
- [ ] First ExecutionModel: Next open fill
- [ ] BDD scenarios locking behavior

**Deliverable**: Full backtest produces reproducible trade list and metrics.

### Phase 4: YOLO Discovery (trendlab-yolo)

**Goal**: Structural Monte Carlo that samples composition first.

- [ ] Genome representation (component types + parameters)
- [ ] Structural sampler (swap components, then jitter params)
- [ ] Warmup phase vs exploitation phase
- [ ] Robustness scoring (cross-symbol, cross-time, cost perturbation)
- [ ] Three leaderboards: Signal Quality, Position Management, Execution Sensitivity
- [ ] Component attribution (what drove performance?)

**Deliverable**: YOLO session produces ranked leaderboard with attribution.

### Phase 5: Terminal UI (trendlab-tui)

**Goal**: Vim-native, YOLO-first interface.

- [ ] Central app state (panels render, don't own)
- [ ] Panels: Home, Results, Chart, Help overlay
- [ ] Vim keybindings: j/k/h/l, gg/G, number keys
- [ ] Semantic color system (status readable at glance)
- [ ] Live leaderboard updates during YOLO
- [ ] One-key export (P exports selected config)

**Deliverable**: `cargo run` → Enter → YOLO runs with live UI.

### Phase 6: Export & Pine Parity (trendlab-export)

**Goal**: Strategies become tradeable Pine scripts.

- [ ] StrategyArtifact JSON (matches schema)
- [ ] Pine Script v6 code generation
- [ ] Parity test vectors (entry/exit dates + prices)
- [ ] Reproducibility bundle (config + seed + manifest)
- [ ] Fill convention documentation in export

**Deliverable**: Exported Pine matches Rust backtest within tolerance.

---

## 6. Testing Strategy

### Test Categories

| Category | Tool | Purpose |
|----------|------|---------|
| BDD scenarios | cucumber-rs | Lock behavioral contracts |
| Property tests | proptest | Invariants hold under random input |
| Determinism tests | seed replay | Same input = same output |
| No-lookahead tests | bar assertions | Bar N sees only bars ≤ N |
| Parity tests | vector comparison | Rust matches Pine output |
| Integration tests | full YOLO | Valid leaderboards produced |

### Key Invariant Tests

1. PositionManager never accesses SignalGenerator internal state
2. `high_since_entry` tracked from ENTRY forward, not globally
3. Exit checks occur BEFORE entry checks per bar
4. Components are fresh per run (no state leakage)
5. Execution assumptions recorded in fingerprint

---

## 7. Success Criteria

| Criterion | Verification |
|-----------|--------------|
| Launch → Enter → YOLO runs | No config required to see results |
| Three leaderboards exist | Signal, PM, Execution sensitivity ranked separately |
| Components freely swap | Any SG × PM × EM × Filter combination works |
| Every run reproducible | Same fingerprint inputs = identical results |
| No stickiness | Exit references explicit, not shared rolling |
| Pine parity | Exported script matches Rust within tolerance |

---

## 8. Agent Delegation

When working on specific domains, delegate to specialized agents:

| Task | Agent |
|------|-------|
| Component traits, stickiness prevention | `core-architect` |
| Simulation loop, trade lifecycle | `backtest-engine` |
| Crate structure, Rust APIs | `rust-architect` |
| Polars pipelines, Parquet I/O | `polars-expert` |
| Strategy families, parameter grids | `trend-following-expert` |
| Yahoo fetch, caching | `data-engineer` |
| BDD scenarios, step definitions | `bdd-test-author` |
| Property tests, benchmarks | `test-engineer` |
| Metrics, ranking logic | `metrics-analyst` |
| Pine Script, parity | `pine-exporter` |
| Monte Carlo, leaderboards | `yolo-researcher` |
| Ratatui UI, keybindings | `tui-developer` |
| Financial charts | `financial-charts-expert` |

---

## 9. Guidance for detailed-plan.md

### When to Create
At the start of each phase. Keep the detailed plan scoped to the active phase
to avoid drift and reduce maintenance.

### Structure

```markdown
# Phase N: [Name] - Detailed Plan

## Files to Create/Modify
- `crates/trendlab-core/src/types.rs` - Bar, Signal, Order types
- ...

## Trait Definitions
```rust
pub trait SignalGenerator {
    fn name(&self) -> &str;
    fn warmup_bars(&self) -> usize;
    fn generate(&mut self, bar: &Bar, state: &MarketState) -> Option<Signal>;
}
```

## Type Definitions
...

## Test Specifications
- Scenario: Signal generator returns None during warmup
- Property: generate() never peeks beyond current bar

## Implementation Order
1. Define types first (Bar, Signal)
2. Define traits second
3. Implement first concrete types
4. Write tests alongside

## Milestone
- [ ] `cargo test -p trendlab-core` passes
- [ ] Property tests for state isolation pass
```

### Phased Expansion
- Keep a Phase 1 detailed plan only
- Move later phases into a separate backlog doc
- Expand the detailed plan only when a phase starts

---

## 10. Current State & Next Steps

### Project State
- **Documentation**: Complete
- **Workspace**: Configured
- **Crates**: Scaffolded (stubs only)
- **Implementation**: ~0.5% complete

### Immediate Next Steps
1. Split detailed plan: Phase 1 only + backlog file for later phases
2. Define core types in `trendlab-core/src/types.rs`
3. Define traits in `trendlab-core/src/traits.rs`
4. Write first property tests
5. Implement first SignalGenerator (Donchian breakout)

---

*Last updated: 2026-01-21*
