# TrendLab vNext Blueprint (Rust + TUI Backtesting Lab)

> **Goal:** Build a research-grade, terminal-native trend-following lab that can explore a *true* universe of strategies in a Monte Carlo setting—**without** structural bias (“stickiness”)—and surface robust winners that survive execution and cost perturbations.

---

## 1) Product thesis

### What vNext must feel like
- **YOLO-first by default:** Launch → Scan → Results → Export.
- **Power-user depth on demand:** every knob exists, but none are required to start.
- **Fast, trustworthy iteration:** results appear quickly *and* are explainable (provenance, run fingerprints, component breakdown).

### The two truths we optimize for
1. **Discovery speed:** find interesting strategy *families* quickly.
2. **Discovery integrity:** prevent “hidden advantages” (especially position-management and execution bundling) from dominating.

---

## 2) Non‑negotiable invariants (the “no stickiness” contract)

These are architecture-level constraints. If any are violated, you will recreate the old failure mode.

### Invariant A — Strategy is not a monolith
A “strategy” is a **composition** of orthogonal components:

- **Signal Generator** (entry/intent)
- **Position Manager** (stops/exits/scaling/time-exit)
- **Execution Model** (fill timing + order semantics + gaps)
- **Filters** (regime/volatility/market filters; optional)

**Rule:** Any signal generator must be able to pair with any position manager and any execution model.

### Invariant B — Signal, Order, Fill are separate layers
Backtesting must be decomposed into:

1) **Signal layer**: compute intents/levels (fast, mostly stateless)  
2) **Order layer**: translate intent → orders (stateful, position-aware)  
3) **Fill layer**: simulate fills & costs (stateful, execution-aware)

### Invariant C — Execution is a first-class randomized variable
YOLO/Monte Carlo must be able to vary:
- fill timing (next-open, close, intrabar assumptions)
- order types (market/stop/limit)
- gap policy
- slippage/fees

### Invariant D — “Sticky exits” require explicit exit reference semantics
Any exit logic that depends on extremes (high/low bands, channel exits, trailing logic) must declare its **exit reference mode**:

- **Entry-frozen reference**
- **Since-entry trailing high/low**
- **Separate entry vs exit lookbacks**

**Rule:** Rolling “shared reference” entry+exit behavior is not allowed by default.

### Invariant E — Comparisons must be disentangled
You do not get “one leaderboard” until you have three:

1) **Signal Quality** (all share same PM + execution)  
2) **Position Management** (all share same signal + execution)  
3) **Execution Sensitivity** (same signal + PM, vary execution)

---

## 3) System overview (high-level architecture)

### Subsystems
- **Data subsystem:** universe definition, fetch, cache, incremental updates
- **Engine subsystem:** signal/order/fill spine, portfolio accounting, metrics
- **Component libraries:** signal generators, position managers, execution models, filters
- **Research subsystem:** structural Monte Carlo, scoring, selection, robustness tests
- **TUI subsystem:** panels, navigation, live progress, results exploration
- **Artifacts subsystem:** export (Pine/JSON/Markdown), run manifests, reproducibility bundles

### Provenance (“why should I trust this result?”)
Every run and every leaderboard row must have:
- a **run fingerprint** (hash of config + seed + component choices)
- component breakdown (signal/PM/execution/filter)
- parameter values
- universe slice + date range
- cost/execution assumptions
- determinism metadata (seed, RNG version)

---

## 4) Research model: Structural Monte Carlo (the real universe)

### What “structural” means
YOLO must explore *structure* first, then parameters:
- swap signal generator
- swap position manager
- swap execution model
- add/remove filters
- then jitter parameters inside the chosen structure

This is how you avoid “parameter MC” that only explores a tiny corner of the space.

### Component pools (strategy genome)
Maintain explicit pools:
- **Signal generators:** MA crossover, Donchian/52Wk breakout, TSMOM, etc.
- **Position managers:** ATR trail, chandelier, time stop, partial exits, etc.
- **Execution models:** next-open market, close-on-signal, stop order fill, limit order fill, etc.
- **Filters:** trend regime, volatility regime, market regime, liquidity, etc.

YOLO samples a “genome” = (signal, PM, execution, filters) + parameters.

---

## 5) Scoring & selection: Robustness first

### Why mean Sharpe is a trap
Averages select fragile systems that fail under small perturbations.

### vNext ranking principle: “Worst-case competence”
Prefer strategies that remain competent across:
- symbols
- time slices / walk-forward windows
- execution/cost assumptions
- random perturbations

**Default scoring approach (conceptual):**
- emphasize lower-tail performance (e.g., 5th percentile)
- penalize drawdown and fragility
- include cost/execution sensitivity penalties

### Built-in robustness checks (default “on”)
- walk-forward or rolling validation window
- hold-out period
- execution perturbation sweep (cheap, fast)
- cost stress (spread/slippage multipliers)

---

## 6) Engine behavior (what must exist, without implementation detail)

### What the engine must simulate
- portfolio-level accounting (equity curve, drawdown, exposures)
- position-level state (entry price, max favorable excursion, trailing refs)
- order lifecycle (created → pending → filled/canceled)
- fill policies (intrabar assumptions consistent and explicit)

### Guardrails (correctness)
- no look-ahead
- warmup handling for indicators
- clear handling for missing data / halts
- survivorship bias awareness (at least “universe is a snapshot” documented)
- event logging that can reconstruct a trade

---

## 7) Data & universe (frictionless by default)

### Defaults
- auto-load a predefined universe at startup
- auto-fetch and cache history
- incremental updates on launch
- cache format optimized for speed and auditability (e.g., columnar)

### Provider pluggability
Data sources should be swappable; the app should not hard-code provider assumptions into the engine.

---

## 8) TUI: Streamlined mode is the default product

### What to remove (default mode)
- manual ticker selection panels
- sweep UI (YOLO is the only default mode)
- deep strategy editing panels

### What to keep
- **Results panel:** real-time leaderboard + filters + sorting
- **Chart panel:** equity curve + drawdown + trade overlays (as modes)
- **Help panel:** searchable docs + keybindings
- **Status bar:** progress + throughput + ETA-free “health”
- **Logging panel or overlay:** important events, warnings, reproducibility info

### Keybinding principles
- vim navigation everywhere (j/k/h/l, gg/G)
- number keys jump to panels
- “?” always opens help
- “P” exports the selected configuration

---

## 9) Artifacts & export (research → action)

### Always exportable
- **Pine script** (or a best-effort mapping)
- **Run manifest** (config + components + assumptions)
- **Metrics summary** (top-level + robustness)
- **Repro bundle** (seed + config + universe + date range)

### Export design goal
A result is only valuable if it can be:
- reproduced
- explained
- deployed (at least as a prototype in TradingView or downstream tooling)

---

## 10) Validation plan (before and during rebuild)

### Fast pre-rewrite diagnostics (optional but recommended)
- **Overlay stop test:** apply the same position manager to multiple signals to see if “winners” were stop-driven.
- **Reference-mode test:** compare rolling reference vs explicit exit reference modes.
- **Regime constraint test:** evaluate whether a simple regime filter equalizes outcomes.
- **Walk-forward sanity check:** verify the current “winner” survives time slicing.

### Continuous validation in vNext
- correctness tests for fills and trade reconstruction
- determinism tests (same seed → same results)
- fairness tests (component leaderboards behave as intended)

---

## 11) Acceptance criteria (Gherkin)

```gherkin
Feature: No more strategy-execution bundle bias
  Scenario: Same entry signal, different position managers
    Given I select a signal generator
    When I pair it with multiple position managers
    Then differences in results are attributable to position management
    And the signal quality leaderboard remains comparable
```

```gherkin
Feature: Stickiness problem resolution
  Background:
    Sticky behavior occurs when entry and exit share a rolling reference.
  Scenario: Exit reference modes are explicit
    Given an extreme-based exit rule is enabled
    Then I must choose an exit reference mode
    And rolling shared-reference is not the default
```

```gherkin
Feature: Execution is a first-class randomized variable
  Scenario: YOLO can randomize execution
    Given randomization includes execution
    When a new iteration is sampled
    Then the execution model may change
    And the run fingerprint records execution assumptions
```

```gherkin
Feature: YOLO-first experience
  Scenario: Launch → Scan → Results → Export
    Given I launch the app
    When I press Enter
    Then structural YOLO begins
    And results appear in real time
    When I select a top configuration and press P
    Then an export artifact is generated
```

```gherkin
Feature: Determinism & provenance
  Scenario: Reproducible runs
    Given I run with a specific seed and config
    When I rerun with the same seed and config
    Then the leaderboard ordering and metrics match exactly
    And trade reconstruction logs are consistent
```

---

## 12) Milestones (definition-of-done oriented)

### Milestone 0 — Blueprint + fairness harness
**DoD**
- Invariants documented and enforced in config schema
- Three leaderboards exist (signal/PM/execution)
- Stickiness reference modes enforced

### Milestone 1 — Order-aware engine spine
**DoD**
- Signal → Order → Fill separation exists end-to-end
- At least two execution models supported with identical strategy genome
- Trade reconstruction and run fingerprint work

### Milestone 2 — Component libraries + genome assembly
**DoD**
- Minimum viable pools: 3 signals, 3 PMs, 2 executions, 2 filters
- Any pairing works without glue code changes
- Component breakdown visible in Results

### Milestone 3 — Structural Monte Carlo + robust scoring
**DoD**
- Structural sampling is live and configurable
- Robustness scoring used for default ranking
- Execution/cost perturbation checks available

### Milestone 4 — Streamlined TUI productization
**DoD**
- Default flow is Launch → Scan → Results → Export
- Help + keybindings + logs are polished
- Export artifacts are stable and discoverable

### Milestone 5 — “Research integrity” hardening
**DoD**
- determinism tests pass
- correctness suite covers fills, stops, gaps, warmups
- documented assumptions + known limitations shipped with the app

---

## 13) Default decisions (so you don’t stall)

- **Default mode:** Streamlined YOLO-only
- **Default ranking:** robustness-first (lower-tail emphasis)
- **Default comparability:** start with standardized PM+execution for signal leaderboard
- **Default transparency:** always show component breakdown + assumptions
- **Default friction rule:** if a user must configure data before seeing results, the UX failed

---

## 14) What success looks like

- You can say: “This strategy family wins *even if* execution worsens, costs rise, and we walk-forward it.”
- You can prove: “It wins because of X (signal) plus Y (position manager), not because we accidentally bundled a superior stop/execution model.”
- The app becomes sticky because **the research loop is addictive**: scan → inspect → export → repeat.

