Below is a “from-the-ground-up” plan for **TrendLab vNext**: keep the TUI superpowers, but rebuild the backtest/strategy layer so it can’t produce the same “stickiness” distortions again.

---

## 1) What we keep (TrendLab’s real strengths)

These are differentiators worth preserving as-is:

* **Terminal-native, vim-style navigation** across panels (muscle memory = speed). 
* **Semantic color + visual hierarchy** (you can “read” status/results instantly). 
* **Help that’s actually usable** (searchable, sectioned, “?” everywhere). 
* **Leaderboard-centric workflow**: watch discovery happen, then drill into winners. 
* **One-button export pipeline** (research → Pine/artifact output). 
* **The streamlined “YOLO-first” product stance**: remove panels and friction; keep results/chart/help/logging. 

---

## 2) What we drop (the root weaknesses behind “stickiness”)

The stickiness issue wasn’t “a bad strategy”—it was structural bias:

* You weren’t comparing strategies; you were comparing **strategy-execution bundles** where Supertrend “wins” largely because it bakes in better position management. 
* The system conflated **signal generation**, **position management**, and **execution mechanics**, so “fair comparisons” were impossible. 
* Execution semantics were too uniform; to explore a real universe, you must vary fill timing/order types/gap behavior/slippage. 

So: we drop the monolithic “strategy does everything” model and replace it with composable parts + an order-aware engine.

---

## 3) Architecture that prevents stickiness by design

### A) The core decomposition: Signal → Order → Fill

This becomes the non-negotiable backbone:

1. **Signal layer** (vectorized, fast): produces intents/levels
2. **Order layer** (stateful adapter): turns intents into orders based on rules
3. **Execution/Fill layer** (simulator): applies fill model, gaps, slippage, fees 

That separation is what lets “execution” become a controlled variable instead of a hidden bias.

### B) Strategy = a composition, not a monolith

A “strategy” in vNext is a *genome* assembled from pools:

* **Signal generator** (entry logic)
* **Position manager** (stops/exits, scaling, time stops)
* **Execution model** (next-open vs close vs stop/limit via OHLC)
* **Filters** (regime suppression)

This matches the “true universe is combinatorial” point: `signal × stop × filter × execution`. 

### C) Fix rolling-reference exits (the “sticky trap”) as a first-class concern

Any proximity-to-extreme method must choose an exit reference mode that does *not* “run away” unfairly. Canonical exit reference modes to bake in:

* **Entry-frozen reference**
* **Since-entry trailing high**
* **Separate entry vs exit lookbacks** 

And importantly: the exit modes that require “memory” live in **position state** (not in the vectorized signal expressions). 

### D) Separate leaderboards so you can tell what’s actually good

You want three distinct “truths,” not one blended ranking:

* **Signal Quality leaderboard** (all signals share same position mgmt + execution)
* **Position Mgmt leaderboard** (all share same entry signal + execution)
* **Execution leaderboard** (same signal + stops, vary fill model)

This makes it impossible for one baked-in stop to dominate the whole lab without you noticing.

---

## 4) YOLO vNext: “structural Monte Carlo” that actually searches the universe

YOLO shouldn’t just jitter parameters; it should sample *structure*:

* Swap signal generator
* Swap position manager
* Swap execution model
* Add/remove filters
  …and then jitter params inside the chosen structure. 

That’s how you get true recombination like: “52-week breakout + ATR trail” → “52-week breakout + chandelier exit” (and discover which *pairings* work). 

**Critical principle:** execution must be a first-class randomized dimension (fill timing/order type/gap policy/slippage), because otherwise you’re still baking in bias. 

---

## 5) Phased build plan (so this ships, not spirals)

### Phase 0 — Lock the rules of fairness

Deliverables:

* A “fairness harness” that can run the same signal with multiple position managers + execution models.
* A “stickiness dashboard”: hold-time percentiles, % trades held > N bars, etc. (so stickiness can’t hide).

**Fast validation test (optional but high value):**

* Add a temporary “overlay stop” concept to prove the hypothesis that position management is driving the winner distribution. 

### Phase 1 — Build the order-aware backtest engine

Deliverables:

* Signal/Order/Fill separation (the spine). 
* Execution models as pluggable “policies” (next-open, close, stop, limit), with gap/slippage/fee policies as inputs.

### Phase 2 — Build the component libraries (your strategy universe)

Deliverables:

* Pools for signal generators, position managers, execution models, filters (so YOLO can recombine).
* Component-level leaderboards + “component stats view” in Results.

### Phase 3 — YOLO structural Monte Carlo + robustness ranking

Deliverables:

* Structural sampling (weights + probabilities), not just parameter sweeps. 
* Robust scoring: focus on stability across symbols and across execution/cost perturbations (not just best Sharpe).

### Phase 4 — Productize the TUI (the “YOLO-only” streamlined app)

Deliverables:

* Remove friction: no Data panel, no Sweep panel; **Launch → Scan → Results → Export**. 
* Keep: Chart, Results, Help, status bar, logging. 

---

## 6) Gherkin “north star” acceptance criteria (sprinkled where it matters)

```gherkin
Feature: No more strategy-execution bundle bias
  Scenario: Same entry signal, different position managers
    Given I select the "52-Week Breakout" signal generator
    When I pair it with multiple position managers
    Then results reflect differences in position management, not hidden strategy coupling
```

```gherkin
Feature: Stickiness is impossible to hide
  Scenario: Rolling-reference exits are not allowed by default
    Given a proximity-to-high exit rule is configured
    Then the system requires an exit reference mode:
      | EntryFrozen | SinceEntryTrailingHigh | SeparateEntryExitLookbacks |
    And the run logs include holding-period percentiles
```

```gherkin
Feature: Execution is a first-class randomized dimension
  Scenario: YOLO randomizes execution model when enabled
    Given YOLO randomization is above the execution threshold
    When a new iteration is sampled
    Then the execution model may change (NextOpen, CloseOnSignal, StopOrder, LimitOrder)
    And the run fingerprint records the chosen execution settings
```

```gherkin
Feature: YOLO-first product experience
  Scenario: Launch → Scan → Results → Export
    Given I launch the application
    When I press Enter
    Then structural YOLO begins
    And the leaderboard updates in real time
    When I select a top configuration
    And I press "P"
    Then the configuration is exported for external use
```

---

If you want, I can translate this plan into a **single “vNext blueprint” markdown** you can drop into the repo (sections, milestones, and the exact non-negotiable invariants that prevent stickiness from returning).
