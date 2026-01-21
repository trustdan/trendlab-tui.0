# Operational Rules for Claude in TrendLab

---

## Pragmatic Programmer Principles

Apply these methodologies throughout all work:

### DRY (Don't Repeat Yourself)

- Every piece of knowledge has a single, unambiguous representation
- If you find yourself copying code, extract it into a shared component
- Documentation that duplicates code will drift — prefer self-documenting code

### Orthogonality

- Components must be independent and interchangeable
- Changes to one component should not require changes to unrelated components
- This is the architectural foundation that prevents "stickiness"

### Tracer Bullets

- Get something working end-to-end early (Signal → Order → Fill → Metrics)
- Use tracer bullets to validate architecture before building out features
- A thin vertical slice beats a thick horizontal layer

### Design by Contract

- Every trait has preconditions, postconditions, and invariants
- SignalGenerators produce intents, not exits
- PositionManagers track state from entry, not globally
- ExecutionModels handle fills, not signals

### Decoupling and the Law of Demeter

- Components talk only through defined interfaces
- No component accesses another component's internal state
- If A depends on B which depends on C, A should not know about C

### Refactor Early, Refactor Often

- Clean up as you go — don't accumulate technical debt
- If something feels wrong, fix it now before it spreads
- Leave the code cleaner than you found it

### Test Early, Test Often, Test Automatically

- Write BDD scenarios before implementation
- Property tests catch edge cases humans miss
- Determinism tests ensure reproducibility

### Fix the Problem, Not the Blame

- When debugging, focus on understanding root cause
- Don't patch symptoms — fix the underlying issue
- Document what went wrong to prevent recurrence

### Prototypes vs Production Code

- Prototypes are disposable — make that clear
- If exploring, label it as exploration
- Production code gets tests, docs, and reviews

---

## Agent Delegation

Delegate immediately when a task matches an agent's domain. Do not attempt complex domain work without consulting the appropriate agent first.

| Task Domain | Agent to Use |
| ----------- | ------------ |
| Component traits, state isolation, stickiness prevention | `core-architect` |
| Simulation loop, event processing, trade lifecycle | `backtest-engine` |
| Crate structure, traits, APIs, Rust architecture | `rust-architect` |
| Polars pipelines, Parquet I/O, performance | `polars-expert` |
| Strategy design, parameter grids, robustness | `trend-following-expert` |
| Data ingestion, Yahoo Finance, caching | `data-engineer` |
| BDD scenarios, cucumber-rs, step definitions | `bdd-test-author` |
| Property tests, integration tests, benchmarks | `test-engineer` |
| Metrics calculations, ranking, cost sensitivity | `metrics-analyst` |
| Pine Script generation, parity testing, artifacts | `pine-exporter` |
| Monte Carlo search, leaderboard ranking, attribution | `yolo-researcher` |
| Ratatui terminal interface, vim keybindings | `tui-developer` |
| Financial charts, candlesticks, indicators | `financial-charts-expert` |

---

## The v1 Failure and Non-Negotiable Invariants

### The Stickiness Problem (Root Cause)

The v1 failure was **structural bias**, not a bad strategy. "Stickiness" occurs when a position becomes impossible to exit because the exit condition moves away faster than price can reach it.

**Example of stickiness:**

> A breakout strategy enters long when price exceeds the rolling 200-day high. The exit rule is "sell when price drops 10% from the rolling high."
>
> - Day 1: Enter at $100 (the 200-day high). Stop is $90.
> - Day 50: Price rallies to $150 (new 200-day high). Stop moves to $135.
> - Day 100: Price rallies to $200. Stop moves to $180.
>
> The stop keeps "running away" because entry and exit share the same rolling reference. The position can only exit on a sharp reversal — it's "sticky."

**Why this produced false winners:**

- Strategies were compared as bundles (signal + exits + execution assumptions)
- A strategy with a superior exit mechanism appeared to have a superior signal
- Fair comparison was impossible because components weren't isolated

### Invariant A — Strategy Is a Composition, Not a Monolith

A strategy MUST be assembled from orthogonal components:

| Layer           | Responsibility                          |
| --------------- | --------------------------------------- |
| SignalGenerator | "Should I consider entering right now?" |
| PositionManager | "How do I manage this trade once in?"   |
| ExecutionModel  | "How does my order get filled?"         |
| SignalFilter    | "Should I suppress this signal?"        |

**Rule:** Any signal generator must pair with any position manager and any execution model.

**Why this matters:** If Strategy A bundles a great exit mechanism and Strategy B bundles a poor one, comparing them tells you nothing about which *signal* is better. Composition forces fair fights.

### Invariant B — Signal → Order → Fill Separation

Backtesting MUST proceed through three conceptual phases:

1. **Signal layer**: compute intents/levels (fast, mostly stateless)
2. **Order layer**: translate intent → orders (stateful, position-aware)
3. **Fill layer**: simulate fills & costs (stateful, execution-aware)

**Why this matters:** If signal code decides fills, or execution code peeks at future bars, results become untrustworthy. Separation enforces that each layer only knows what it should know.

### Invariant C — Exit Reference Semantics Are Explicit

Any exit rule based on price extremes (highs, lows, bands, channels) MUST declare how the reference point is determined:

| Mode | Meaning | Example |
| ---- | ------- | ------- |
| **EntryFrozenReference** | Reference is fixed at entry and never updates | "Stop at 10% below the high *on the day I entered*" |
| **SinceEntryTrailingExtreme** | Reference tracks the extreme *since entry*, not globally | "Stop at 10% below the highest price *since I entered*" |
| **SeparateEntryExitLookbacks** | Entry uses one window, exit uses another | "Enter on 200-day high breakout, exit on 50-day low breakdown" |

**The forbidden pattern:** Using the same rolling reference for both entry AND exit (this causes stickiness). If not explicitly chosen, the system must refuse to run.

### Invariant D — Execution Is a First-Class Variable

ExecutionModel must control and YOLO must be able to randomize:

- Fill timing (next open, close, intrabar assumptions)
- Order types (market/stop/limit)
- Gap policy (what happens when price gaps through a stop?)
- Slippage/fees/spread models

**Why this matters:** A strategy that looks great with "fill at close" might fall apart with "fill at next open + slippage." If execution is hardcoded, you can't test robustness to realistic trading conditions.

### Invariant E — Comparisons Must Be Disentangled

You cannot trust an "overall winner" until you have three separate leaderboards:

| Leaderboard | What varies | What's fixed | What it tells you |
| ----------- | ----------- | ------------ | ----------------- |
| **Signal Quality** | Signal generators | Same PM + execution for all | Which *signal* is best |
| **Position Management** | Position managers | Same signal + execution for all | Which *exit method* is best |
| **Execution Sensitivity** | Execution models | Same signal + PM for all | How fragile results are to fill assumptions |

**Why this matters:** Without separation, you might crown a "winning strategy" that actually has a mediocre signal but a great exit mechanism. The disentangled leaderboards reveal which *component* is actually driving performance.

### Invariant F — Determinism & Provenance

Every run emits:

- **Run fingerprint** (hash of genome + params + universe + date range + execution/cost assumptions + seed)
- **Component breakdown** (signal/PM/execution/filter)
- **Manifest** (all assumptions readable)

**Why this matters:** If you can't reproduce a result exactly, you can't trust it. If you can't see the assumptions, you can't evaluate whether they're realistic.

---

## Component Contract Rules

### SignalGenerator Contract

- MUST implement: `name()`, `warmup_bars()`, `generate(bar, market_state) → Option<Signal>`, `parameter_spec()`
- MUST NOT: track position state, know about current holdings, make exit decisions
- State is per-indicator, not per-trade

### PositionManager Contract

- MUST implement: `on_entry(bar, price)`, `on_bar(bar, position) → Action`, `stop_price()`, `parameter_spec()`
- MUST initialize state at entry (e.g., `high_since_entry = entry_price`)
- MUST NOT access SignalGenerator's internal state
- Action can be: Hold, AdjustStop(price), ScaleOut(pct), Exit

### ExecutionModel Contract

- MUST implement: `attempt_fill(signal, current_bar, next_bar) → FillResult`, `gap_policy()`
- MUST explicitly declare fill timing + gap + slippage/fees semantics
- FillResult includes: filled (bool), fill_price, slippage

### SignalFilter Contract

- MUST implement: `allow_signal(signal, bar, market_state) → bool`, `force_exit(position, bar, market_state) → bool`
- Filters can suppress entries OR force exits (regime changes)

---

## YOLO / Monte Carlo Rules

### Structural Sampling (Not Just Parameter Jitter)

YOLO must explore **structure** first, then parameters:

1. Swap signal generator
2. Swap position manager
3. Swap execution model
4. Add/remove filters
5. THEN jitter parameters inside the chosen structure

### Three-Layer Monte Carlo

1. **Structural Sampling**: randomly assemble a genome
2. **Parameter Jitter**: fuzz inputs within defined ranges
3. **Execution Noise**: run with variable slippage/gap fills to punish fragile "perfect" backtests

### Robustness Scoring (Not Mean Sharpe)

Prefer strategies that remain competent across:

- Symbols
- Time slices / walk-forward windows
- Execution/cost perturbations

Default scoring includes:

- Lower-tail emphasis (5th percentile matters)
- Drawdown penalties
- Fragility penalties (sensitivity to costs/execution)
- Symbol hit rate (% of symbols with Sharpe > 0)

---

## Code Quality Gates

Before any PR or significant commit:

1. `cargo fmt` — must pass
2. `cargo clippy --all-targets --all-features -D warnings` — must pass
3. `cargo test` — must pass (including BDD tests)

Use `/dev:release-check` to run all gates and produce a summary.

---

## Test-First Development

1. Write or extend `.feature` scenarios BEFORE implementing behavior
2. Never weaken tests to make code pass
3. Use fixtures from `fixtures/` — keep them small (20-200 bars)
4. Required invariant tests:
   - No lookahead (bar N sees only bars ≤ N)
   - Accounting identities (equity = cash + positions)
   - Determinism (same seed → same results)
   - State isolation (components cannot access each other's internals)
   - Position manager tracks from entry, not globally

---

## Data Handling

### Invisible Data Layer (No User Friction)

- Data fetching happens automatically in the background
- Default universe loads at startup
- Incremental sync only (delta updates)
- User is never blocked by data management

### Technical Rules

- **Never** read market data with eager Polars methods — use `scan_parquet` (lazy)
- **Never** commit anything in `data/` — it's gitignored
- **Always** use fixtures for tests, not real market data
- **Always** log data provenance (provider, fetch timestamp, version)

---

## TUI Principles

### YOLO-First Product Stance

- Launch → Press Enter → structural YOLO begins
- Configuration is optional (modal for power users)
- Results appear live (leaderboard updates)
- **If the user must configure data before seeing results, UX failed.**

### Preserved Strengths

- Vim-native navigation (j/k/h/l, gg/G, number keys for panels)
- Semantic color system (status readable at a glance)
- Context-sensitive help (? everywhere)
- One-key export (P exports selected config)

### Simplified Panel Structure

- Four panels max (Home, Results, Chart, Help overlay)
- Remove friction: no Data panel, no Sweep panel in default mode
- Help is an overlay, not a panel

---

## Pine Parity

Every strategy that might become a Pine script must:

1. Emit a `StrategyArtifact` JSON (schema in `schemas/`)
2. Include parity test vectors (timestamps + expected values)
3. Document fill convention and cost model explicitly
4. Map Rust traits directly to Pine functions (clean, modular output)

---

## Assumptions Are Code

When changing any assumption (fill convention, timezone, adjusted prices), you must:

1. Update `docs/assumptions.md`
2. Add or update BDD scenarios that enforce the assumption
3. Review all code that depends on the assumption

---

## Common Mistakes to Avoid

### Architecture Violations

- Creating monolithic "strategy" types that bundle signal + exits
- Using rolling shared references for entry AND exit
- Allowing components to access each other's internal state
- Skipping the Signal → Order → Fill separation

### Data Mistakes

- Using eager Polars reads instead of `scan_parquet`
- Adding tests that depend on live market data
- Creating files in `data/` that should be in `fixtures/`

### Testing Mistakes

- Weakening tests to make failing code pass
- Changing assumptions without updating docs and tests
- Testing only happy paths, not edge cases

### Process Mistakes

- Optimizing for mean Sharpe instead of robustness
- Ranking by peak performance instead of consistency
- Trusting "overall winners" before disentangled leaderboards exist

---

## Progress Reporting (Pacman Style)

When working on multi-step tasks, periodically output progress bars in Arch Linux pacman style to keep the user informed:

```text
:: Processing task...
 [###############-------]  65%  Building component traits
 [######################] 100%  Completed signal generator
```

**Format rules:**

- Use `::` prefix for task group headers
- Square brackets `[` `]` containing the progress
- Hash marks `#` for completed portion, dashes `-` for remaining
- Right-aligned percentage
- Brief description of current step
- For numbered steps: `(1/5) step name [######--------------]  30%`

**When to output progress:**

- At the start of each major phase
- When completing significant milestones
- During long-running operations (every 3-5 steps)
- When transitioning between components or files

**Example multi-step task:**

```text
:: Implementing new PositionManager...
(1/4) defining trait impl     [######################] 100%
(2/4) adding state tracking   [######################] 100%
(3/4) writing tests           [##########------------]  45%
(4/4) updating docs           [----------------------]   0%
```

---

## Quick Decision Guide

| Situation | Action |
| --------- | ------ |
| "Should signal generator know about exits?" | NO — that's PositionManager's job |
| "Can I use rolling reference for entry AND exit?" | NO — declare explicit exit reference mode |
| "Should I rank by highest CAGR?" | NO — use robustness scoring |
| "Can components share mutable state?" | NO — communicate through defined interfaces |
| "Is this a prototype or production?" | Decide upfront; prototypes are disposable |
| "Should I fix this later?" | NO — refactor early, refactor often |
