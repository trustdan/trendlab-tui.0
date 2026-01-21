# TrendLab v2: Project Intelligence

> **Mission:** Build a research-grade, terminal-native trend-following lab in Rust (Ratatui) that explores a *true* strategy universe via structural Monte Carlo (YOLO) and surfaces **robust** winners—without hidden structural bias (the v1 "stickiness" failure mode).

---

## TL;DR

TrendLab v1 failed because "strategies" were **bundles**: signal + exits + implicit execution assumptions. That made comparison unfair and produced "sticky" behavior (rolling references used for both entry and exit).

TrendLab v2 prevents this by design:

- **Strategy = composition** of orthogonal components
- **Signal → Order → Fill** is a strict engine spine
- **Exit reference semantics are explicit** (no accidental rolling shared references)
- **Execution is a first-class variable** (and can be randomized)
- **Multiple leaderboards** exist to disentangle what's actually winning
- **Every result is reproducible** (run fingerprint + seed + manifest)

---

## The v1 Failure and the v2 Cure

### What went wrong (stickiness)

Certain strategies (e.g., 52-week breakout) used the same rolling reference for **entry and exit**, creating a runaway condition where exits "moved away" with new highs/lows.

### What must be true (the cure)

- Signals *never* provide exit references
- Position management state starts **at entry** and lives **inside the PositionManager**
- Anything extreme-based must declare an **exit reference mode**

---

## Composition Architecture

Every strategy is assembled from four independent layers:

| Layer           | Responsibility                                            |
| --------------- | --------------------------------------------------------- |
| SignalGenerator | "Should I consider entering? At what level/intent?"       |
| PositionManager | "How do I manage this trade once in?"                     |
| ExecutionModel  | "How do orders become fills under realistic assumptions?" |
| SignalFilter    | "Should I suppress/allow the signal in this regime?"      |

The structural space is the product. YOLO samples *structure first*, then parameters.

> If you cannot freely recombine these layers, you're rebuilding v1.

---

## Non-Negotiable Invariants

These are architecture-level constraints. If violated, results become untrustworthy.

### A — Strategy is not a monolith

A "strategy" MUST be a composition of independent components.

- No component may depend on another component's internal state
- No "super-strategy" types that implicitly bundle exits/execution

### B — Signal → Order → Fill separation

Backtesting MUST proceed through three conceptual phases:

1. **Signal**: compute intent/levels from historical bars (no trade state)
2. **Order**: translate intent + current position state into orders
3. **Fill**: apply execution assumptions (gaps, slippage, order types, timing)

If "signal code" decides fills, or "execution code" peeks at future bars, the engine is wrong.

### C — Exit reference semantics must be explicit

Any extreme-based exit MUST choose one:

- **EntryFrozenReference** (reference fixed at entry)
- **SinceEntryTrailingExtreme** (tracked from entry forward)
- **SeparateEntryExitLookbacks** (distinct windows)

Rolling "shared reference" entry+exit behavior is NOT the default.

### D — Execution is a first-class variable

ExecutionModel must control:

- Fill timing (next open / close / intrabar policy)
- Order types (market/stop/limit)
- Gap policy
- Slippage/fees/spread models

### E — Multiple leaderboards

At minimum, maintain:

1. **Signal Quality leaderboard** (fixed PM + fixed execution)
2. **Position Management leaderboard** (fixed signal + fixed execution)
3. **Execution Sensitivity leaderboard** (fixed signal + fixed PM, vary execution)

### F — Determinism & provenance

Every run must emit:

- **Run fingerprint** (hash of genome + params + universe + date range + execution/cost assumptions + seed)
- **Manifest** (all assumptions readable)
- Ability to reproduce exactly given the same fingerprint inputs

---

## Project Structure

```text
trendlab-v2/
├── crates/
│   ├── trendlab-core/        # Engine spine, traits, types, metrics
│   ├── trendlab-data/        # Fetch/cache/universe (invisible infra)
│   ├── trendlab-yolo/        # Structural Monte Carlo, ranking, attribution
│   ├── trendlab-tui/         # Ratatui UI, panels, keybindings
│   └── trendlab-export/      # Pine + artifacts (JSON/MD) + repro bundles
├── artifacts/
│   ├── sessions/             # YOLO session logs/manifests
│   └── exports/              # strategy exports + repro bundles
├── pine-scripts/             # generated Pine scripts
└── Cargo.toml                # workspace root
```

---

## Product Stance: YOLO-First

- Launch → Press Enter → structural YOLO begins
- Configuration is optional (modal for power users)
- Results appear live (leaderboard updates)
- Selecting a row reveals component breakdown + assumptions
- "Export" produces Pine + manifest + metrics summary

**If the user must configure data before seeing results, UX failed.**

---

## Ranking Philosophy

Prefer strategies that remain competent across:

- Symbols
- Time slices / walk-forward windows
- Execution/cost perturbations

Default ranking includes:

- Drawdown penalties
- Fragility penalties (sensitivity to costs/execution)
- Lower-tail emphasis (bad-case behavior matters)

**Avoid ranking purely by mean Sharpe/Return.**

---

## Development Workflows

### New SignalGenerator

- MUST produce only entry intents/levels (no exit logic)
- MUST not store cross-run state
- MUST not peek beyond bar N

### New PositionManager

- MUST initialize state at entry (e.g., high_since_entry)
- MUST declare exit reference mode where applicable
- MUST output actions only (hold/adjust/exit), not direct fills

### New ExecutionModel

- MUST explicitly declare fill timing + gap + slippage/fees semantics
- MUST be eligible for YOLO randomization (when enabled)

### TUI Work

- App state lives centrally; panels render state, don't own it
- Keybindings dispatch actions (not ad-hoc mutations)
- Semantic color system is consistent across panels

### YOLO Features

- Sampling must support structure + parameters
- Must record sampled genome + seed into the run fingerprint
- Must support attribution views (what component drove performance?)

---

## Testing Philosophy

Required test categories:

- **Property tests:** invariants (no cross-component state access; fresh state per run)
- **Determinism tests:** same seed/config → identical results
- **No-lookahead tests:** bar N sees only bars ≤ N
- **Execution semantics tests:** stop/limit behavior, gap handling, intrabar assumptions
- **Regression tests:** known configs produce stable metrics across refactors
- **Integration tests:** full YOLO session produces valid, attributable leaderboards

---

## Critical Invariants Checklist

1. PositionManagers never access SignalGenerator internal state
2. Anything like `high_since_entry` is tracked from ENTRY forward
3. No lookahead bias (bar N sees only bars 0..=N)
4. Exit checks occur BEFORE entry checks per bar
5. Components are fresh per run (no state leakage across backtests)
6. Execution assumptions are explicit and recorded in fingerprints/manifests
7. Three leaderboards exist before "overall winners" are trusted

---

## Agents

Delegate immediately when a task matches an agent's domain.

| Domain                                               | Agent                     |
| ---------------------------------------------------- | ------------------------- |
| Component traits, state isolation, stickiness        | `core-architect`          |
| Simulation loop, event processing, trade lifecycle   | `backtest-engine`         |
| Crate structure, APIs, Rust architecture             | `rust-architect`          |
| Polars pipelines, Parquet I/O, performance           | `polars-expert`           |
| Strategy families, parameter grids, robustness       | `trend-following-expert`  |
| Data ingestion, Yahoo Finance, caching               | `data-engineer`           |
| BDD scenarios, cucumber-rs, step definitions         | `bdd-test-author`         |
| Property tests, integration tests, benchmarks        | `test-engineer`           |
| Metrics calculations, ranking, cost sensitivity      | `metrics-analyst`         |
| Pine Script generation, parity testing, artifacts    | `pine-exporter`           |
| Monte Carlo search, leaderboard ranking              | `yolo-researcher`         |
| Ratatui terminal interface, vim keybindings          | `tui-developer`           |
| Financial charts, candlesticks, indicators           | `financial-charts-expert` |

Keep agents specialized. If an agent proposal blurs boundaries, push back.

---

## Progress Reporting

When working on tasks, output pacman-style progress bars to keep the user informed:

```text
:: Synchronizing codebase analysis...
 scanning traits         [######################] 100%
 analyzing components    [###########-----------]  52%
```

**Format:**

- `::` prefix for task headers
- `[####----]` style bars (# = done, - = remaining)
- Right-aligned percentage + brief description
- Numbered format for sequential steps: `(2/5) step [####--] 40%`

Output progress at major milestones, phase transitions, and during long operations.

---

## Quick Routing

| Question                                 | Agent                              |
| ---------------------------------------- | ---------------------------------- |
| "Is this an architecture violation?"     | `core-architect`                   |
| "Why did this trade fill incorrectly?"   | `backtest-engine`                  |
| "Leaderboard ranking feels wrong"        | `yolo-researcher`                  |
| "UI state is spaghetti"                  | `tui-developer`                    |
| "Data missing or weird"                  | `data-engineer`                    |
| "Pine doesn't match backtest"            | `pine-exporter`                    |
| "How to test this invariant?"            | `test-engineer` + `bdd-test-author`|
| "Performance is slow"                    | `polars-expert`                    |
| "What strategy families to test?"        | `trend-following-expert`           |
