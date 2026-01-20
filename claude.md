Here’s a refined **CLAUDE.md v2** that tightens the “stickiness” fix into **hard invariants**, makes **execution + fills** first-class (so you don’t accidentally re-bundle advantages), and adds **provenance + leaderboards** so results stay interpretable as the search space explodes.

```markdown
# TrendLab v2: Project Intelligence (CLAUDE.md v2)

> **Mission:** Build a research-grade, terminal-native trend-following lab in Rust (Ratatui) that explores a *true* strategy universe via structural Monte Carlo (YOLO) and surfaces **robust** winners—without hidden structural bias (the v1 “stickiness” failure mode).

---

## 0) TL;DR (What you must internalize)

TrendLab v1 failed because “strategies” were effectively **bundles**: signal + exits + implicit execution assumptions. That made comparison unfair and produced “sticky” behavior (rolling references used for both entry and exit).

TrendLab v2 prevents this by design:

- **Strategy = composition** of orthogonal components  
- **Signal → Order → Fill** is a strict engine spine  
- **Exit reference semantics are explicit** (no accidental rolling shared references)  
- **Execution is a first-class variable** (and can be randomized)  
- **Multiple leaderboards** exist to disentangle what’s actually winning  
- **Every result is reproducible** (run fingerprint + seed + manifest)

---

## 1) What is this project?

TrendLab v2 is a **trend-following backtesting research lab**:
- written in **Rust**
- with a **terminal UI** (Ratatui)
- designed for **structural Monte Carlo** exploration (“YOLO mode”)
- produces **explainable, reproducible** results and export artifacts (Pine + JSON manifests)

---

## 2) The v1 failure (Stickiness) and the v2 cure

### v1 stickiness (what went wrong)
Certain strategies (e.g., 52-week breakout) used the same rolling reference for **entry and exit**, creating a runaway condition where exits “moved away” with new highs/lows.

### v2 cure (what must be true)
- Signals *never* provide exit references.
- Position management state starts **at entry** and lives **inside the PositionManager**.
- Anything extreme-based must declare an **exit reference mode** (see invariants).

---

## 3) The Composition Architecture (the “strategy genome”)

Every strategy is assembled from four independent layers:

| Layer | Responsibility |
|------|----------------|
| SignalGenerator | “Should I consider entering? At what level/intent?” |
| PositionManager | “How do I manage this trade once in?” |
| ExecutionModel | “How do orders become fills under realistic assumptions?” |
| SignalFilter | “Should I suppress/allow the signal in this regime?” |

**Important:** the structural space is the product. YOLO samples *structure first*, then parameters.

> If you cannot freely recombine these layers, you’re rebuilding v1.

---

## 4) Non-negotiable invariants (the “no stickiness” contract)

These are architecture-level constraints. If violated, results become untrustworthy.

### Invariant A — Strategy is not a monolith
A “strategy” MUST be a composition of independent components.
- No component may depend on another component’s internal state.
- No “super-strategy” types that implicitly bundle exits/execution.

### Invariant B — Signal → Order → Fill separation (engine spine)
Backtesting MUST proceed through three conceptual phases:

1) **Signal**: compute intent/levels from historical bars (no trade state)  
2) **Order**: translate intent + current position state into orders  
3) **Fill**: apply execution assumptions (gaps, slippage, order types, timing)

If “signal code” decides fills, or “execution code” peeks at future bars, the engine is wrong.

### Invariant C — Exit reference semantics must be explicit
Any extreme-based exit (channels, trailing extremes, breakouts-as-exits, etc.) MUST choose one:

- **EntryFrozenReference** (reference fixed at entry)
- **SinceEntryTrailingExtreme** (tracked from entry forward)
- **SeparateEntryExitLookbacks** (distinct windows; still must avoid shared rolling refs)

**Rule:** Rolling “shared reference” entry+exit behavior is NOT the default and must be intentional.

### Invariant D — Execution is a first-class variable
Execution assumptions are part of the experiment, not a hidden constant.
ExecutionModel must control:
- fill timing (next open / close / intrabar policy)
- order types (market/stop/limit)
- gap policy
- slippage/fees/spread models

### Invariant E — Comparisons must be disentangled (multiple leaderboards)
At minimum, maintain:
1) **Signal Quality leaderboard** (fixed PM + fixed execution)
2) **Position Management leaderboard** (fixed signal + fixed execution)
3) **Execution Sensitivity leaderboard** (fixed signal + fixed PM, vary execution)

A single “overall” leaderboard is allowed only when the above exist.

### Invariant F — Determinism & provenance are mandatory
Every run must emit:
- **run fingerprint** (hash of genome + params + universe + date range + execution/cost assumptions + seed)
- **manifest** (all assumptions readable)
- ability to reproduce exactly given the same fingerprint inputs

---

## 5) Product stance: YOLO-first experience

Default flow:
- Launch → Press Enter → structural YOLO begins
- Configuration is optional (modal for power users)
- Results appear live (leaderboard updates)
- Selecting a row reveals component breakdown + assumptions
- “Export” produces Pine + manifest + metrics summary

**If the user must configure data before seeing results, UX failed.**

---

## 6) Project structure (workspace)

```

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

````

---

## 7) Data is infrastructure (not UI)

- Auto-fetch on startup
- Parquet cache (default: `~/.trendlab/data/`)
- incremental updates
- “Universe” is a named concept (e.g., SP500, Liquid ETFs, custom lists)
- engine should not know *how* data is fetched—only how it’s requested

---

## 8) Ranking philosophy: robustness-first (avoid “pretty backtests”)

We prefer strategies that remain competent across:
- symbols
- time slices / walk-forward windows
- execution/cost perturbations

**Avoid ranking purely by mean Sharpe/Return.**
Default ranking should include:
- drawdown penalties
- fragility penalties (sensitivity to costs/execution)
- lower-tail emphasis (bad-case behavior matters)

---

## 9) Development workflows (how to add things safely)

### When implementing a new SignalGenerator
- MUST produce only entry intents/levels (no exit logic)
- MUST not store cross-run state
- MUST not peek beyond bar N

### When implementing a new PositionManager
- MUST initialize state at entry (e.g., high_since_entry)
- MUST declare exit reference mode where applicable
- MUST output actions only (hold/adjust/exit), not direct fills

### When adding an ExecutionModel
- MUST explicitly declare fill timing + gap + slippage/fees semantics
- MUST be eligible for YOLO randomization (when enabled)

### When working on the TUI
- App state lives centrally; panels render state, don’t own it
- Keybindings dispatch actions (not ad-hoc mutations)
- Semantic color system is consistent across panels

### When adding YOLO features
- Sampling must support structure + parameters
- Must record sampled genome + seed into the run fingerprint
- Must support attribution views (what component drove performance?)

---

## 10) Testing philosophy (what we test and why)

Minimum required test categories:

- **Property tests:** invariants (no cross-component state access; fresh state per run)
- **Determinism tests:** same seed/config → identical results
- **No-lookahead tests:** bar N sees only bars ≤ N
- **Execution semantics tests:** stop/limit behavior, gap handling, intrabar assumptions
- **Regression tests:** known configs produce stable metrics across refactors
- **Integration tests:** full YOLO session produces valid, attributable leaderboards

---

## 11) Critical invariants checklist (quick reference)

1. PositionManagers never access SignalGenerator internal state  
2. Anything like `high_since_entry` is tracked from ENTRY forward  
3. No lookahead bias (bar N sees only bars 0..=N)  
4. Exit checks occur BEFORE entry checks per bar (document and enforce)  
5. Components are fresh per run (no state leakage across backtests)  
6. Execution assumptions are explicit and recorded in fingerprints/manifests  
7. Three leaderboards exist (signal / PM / execution) before “overall winners” are trusted

---

## 12) Gherkin acceptance criteria (living spec)

```gherkin
Feature: Stickiness is impossible by default
  Scenario: Extreme-based exits require explicit reference mode
    Given an extreme-based exit is enabled
    Then the system requires an exit reference mode selection
    And rolling shared-reference is not the default
````

```gherkin
Feature: Components are recombinable
  Scenario: Any signal can pair with any position manager and execution model
    Given a set of signal generators
    And a set of position managers
    And a set of execution models
    When I sample a strategy genome
    Then the genome is valid without bespoke glue code
```

```gherkin
Feature: Execution is first-class and attributable
  Scenario: YOLO varies execution assumptions when enabled
    Given YOLO randomization includes execution
    When a new iteration is sampled
    Then the execution model may change
    And the run fingerprint records those assumptions
```

```gherkin
Feature: Deterministic reproducibility
  Scenario: Same inputs reproduce identical outputs
    Given a specific seed and configuration
    When I rerun with the same seed and configuration
    Then results match exactly
    And trade reconstruction is identical
```

---

## 13) Agents (who to ask for what)

* **@core-architect**: invariants, type system, composition boundaries
* **@backtest-engine**: simulation spine, event ordering, metrics correctness
* **@execution-modeler**: fill semantics, gaps, order lifecycle, realism knobs
* **@tui-developer**: Ratatui panels, keybindings, UI architecture
* **@data-engineer**: fetch, cache, universe definitions, incremental sync
* **@yolo-researcher**: sampling, ranking, attribution, robustness
* **@pine-exporter**: Pine v6 generation, parity testing, export artifacts
* **@test-engineer**: property/integration/determinism/regression suites

(Keep agents specialized. If an agent proposal blurs boundaries, push back.)

---

## 14) Getting help (routing)

* “Is this architecture violation?” → @core-architect
* “Why did this trade fill?” → @execution-modeler + @backtest-engine
* “Leaderboard feels wrong” → @yolo-researcher
* “UI state is spaghetti” → @tui-developer
* “Data missing/weird” → @data-engineer
* “Pine doesn’t match backtest” → @pine-exporter
* “How to test this invariant?” → @test-engineer

```

### What I changed vs your original
- Promoted the most important ideas into **non-negotiable invariants** (so future work can’t “accidentally” regress into v1).
- Added the missing spine: **Signal → Order → Fill**, which is where execution bias usually sneaks in.
- Added the concept of **three leaderboards** (signal/PM/execution) so “winners” stay interpretable.
- Made **determinism + run fingerprints** mandatory (research tooling stays trustworthy).
- Added an **execution-modeler** agent role because fills/slippage/gaps are where a lot of “silent advantage” hides.

If you want, paste your actual repo’s current CLAUDE.md (if it differs from this snippet) and I’ll do a tighter diff-style v2 that preserves your wording where possible while still enforcing the invariants.
```
