This is a comprehensive blueprint for **TrendLab Evolution** (let's call it **TLE**).

This plan moves the application from a "Backtesting Calculator" (where you input specific settings to check them) to a "Strategy Discovery Engine" (where you define high-level goals and the machine finds the robust architecture for you).

---

# Product Vision: The "YOLO-First" Research Engine

**Philosophy:**

1. **Input Goals, Not Parameters:** Users shouldn't be guessing moving average lengths. They should be defining risk tolerance and universes.
2. **Combinatorial Discovery:** The app explores the *structure* of trading (Entry × Exit × Execution), not just the settings.
3. **Robustness is King:** A strategy is only a winner if it survives parameter jitter, execution noise, and regime changes.

### The "One-Minute" User Story

```gherkin
Feature: Zero-Friction Discovery

  Scenario: From Launch to Pine Script
    Given I launch TLE
    And I accept the default universe (S&P 500 + Crypto)
    When I press "Enter" to start the "True Monte Carlo" engine
    Then the app automatically:
      - Fetches/updates missing data in the background
      - Begins structural exploration (Signals × Exits × Filters)
      - Surfaces "Robustness Leaders" on the live leaderboard
    When I select the top result and press "P"
    Then a fully valid Pine Script v6 is generated

```

---

# I. Architecture: The Decomposed Engine

To fix the "stickiness" problem and enable true discovery, we must destroy the concept of a monolithic "Strategy" and replace it with **Composable Primitives**.

### 1. The Strategy Genome (Traits & Components)

Instead of a `Supertrend` struct, a Strategy is now a container holding three swappable traits. This ensures fair fights—any entry signal can now wield a superior exit mechanism.

* **Trait 1: The Signal Generator (The "When")**
* *Responsibility:* Pure market timing. Emits `Long/Short/Neutral` intent.
* *Examples:* `Breakout(52wk)`, `Momentum(Roc)`, `Regime(MA_Cross)`.
* *Fix:* No exit logic lives here. Just entry triggers.


* **Trait 2: The Position Manager (The "How")**
* *Responsibility:* Once in a trade, manages risk. Calculates stop levels, targets, and time-limits.
* *Examples:* `AtrTrail` (The Supertrend magic), `ChandelierExit`, `FixedPercent`, `TimeDecay`.
* *Fix:* This solves "stickiness." A 52-week high entry can now use a Chandelier Exit, preventing the stop from racing away with the trend.


* **Trait 3: The Execution Handler (The "Reality")**
* *Responsibility:* Simulates the mechanics of filling the order.
* *Examples:* `StopLimit` (breakouts), `MarketOnClose`, `NextOpen`.



```gherkin
Feature: Combinatorial Architecture

  Scenario: De-coupling solves unfair advantages
    Given the engine is exploring "Breakout Strategies"
    When it tests "52-Week High" entry logic
    Then it automatically permutes it against:
      - An ATR Trailing Stop (Supertrend style)
      - A Fixed % Trail (Legacy style)
      - A Volatility Chandelier (Adaptive style)
    And the leaderboard reveals the "52-Week High + ATR Trail" combo is a winner
    Resulting in a fair comparison vs Supertrend

```

### 2. The "True Monte Carlo" Runner

The engine no longer just jitters numbers. It performs **Structural Mutation**.

* **Layer 1: Structural Sampling:** Randomly assembles a genome (e.g., *Donchian Entry* + *Volatility Filter* + *ATR Exit*).
* **Layer 2: Parameter Jitter:** Once a structure is chosen, it fuzzes the inputs (e.g., lookback 19-25, ATR mult 2.5-3.5).
* **Layer 3: Execution Noise:** It runs the same setup multiple times with variable slippage and randomized gap fills to punish fragile "perfect" backtests.

---

# II. User Experience: Strength Retention & Streamlining

We keep the VIM DNA but remove the administrative overhead.

### 1. The "Invisible" Data Layer

**Weakness Removed:** No more "Data Panel" where you manually select tickers and date ranges.
**New Flow:**

* On startup, TLE loads a "Default Universe" (e.g., liquid ETFs, top Crypto, Tech stocks).
* It performs an incremental sync (updates only yesterday's data) in the background.
* The user is never blocked by data management.

### 2. The Unified Interface (Tabs)

The UI is flattened into three primary views, accessible via `1`, `2`, `3`.

* **View 1: The Lab (Live Research)**
* *Visuals:* A matrix of active workers. Real-time log of "New Best Found."
* *Controls:* `s` to Start/Stop. `c` for high-level Config (Risk profile, Asset class).


* **View 2: The Leaderboard (Results)**
* *Visuals:* Ranked list of **Genomes**, not just parameter sets.
* *Columns:* "Robustness Score" (primary sort), CAGR, Drawdown, "Survival Rate" (how often it worked across Monte Carlo paths).
* *Sub-View:* "Component Stats" – Shows which *traits* are winning (e.g., "ATR Exits are winning 80% of the time, regardless of Entry signal").


* **View 3: The Microscope (Deep Dive)**
* *Visuals:* Equity curves, drawdown charts, and "Cone of Uncertainty" (Monte Carlo paths).
* *Action:* Detailed inspection of a single selected genome from the Leaderboard.



```gherkin
Feature: TUI Streamlining

  Scenario: Component Analysis View
    Given I am viewing the Leaderboard
    When I press "v" to switch to "Component View"
    Then I see a breakdown of "Entry Signal Performance"
    And I see that "Donchian Channels" have a 15% higher median Sharpe than "MA Cross"
    But "Fixed Percent Stops" have a 50% higher drawdown than "ATR Stops"
    Allowing me to understand *why* strategies are winning

```

---

# III. The "Robustness" Metric (The New Score)

We stop optimizing for the highest CAGR (which finds lucky outliers) and start optimizing for **Robustness**.

**The Robustness Score (0-100) is calculated by:**

1. **Parameter Stability:** Does performance crash if I change the lookback from 20 to 21?
2. **Execution Resilience:** Does it still make money if slippage doubles?
3. **Regime Consistency:** Does it work on both Crypto (High Vol) and Bonds (Low Vol)?

*Only strategies with high Robustness Scores allow for Pine Script export.*

---

# IV. Strategic Development Plan

### Phase 1: The Core Rewrite (The Engine)

* **Goal:** Build the `Genome` struct and the `Trait` system.
* **Action:** Implement the "Stickiness Fix" (separate Entry/Exit logic).
* **Test:** Verify that a `52WeekHigh` entry paired with an `AtrTrail` exit performs competitively against `Supertrend`.

### Phase 2: The "True Monte Carlo" Worker

* **Goal:** Build the randomizer that mutates structures, not just params.
* **Action:** Create the `ExecutionHandler` to simulate stop/limit orders properly.
* **Test:** Ensure the engine can run "YOLO mode" and generate thousands of unique structural combinations per minute.

### Phase 3: The TUI Facelift

* **Goal:** Build the 3-Tab unified interface.
* **Action:** Port the Vim keybindings and Color system. Implement the "Invisible Data Layer."
* **Test:** "The One Minute Test" — Can a user launch and export a Pine script in under 60 seconds?

### Phase 4: The Bridge (Export)

* **Goal:** Perfect the Pine Script generation.
* **Action:** Map the Rust `Traits` directly to Pine functions.
* **Result:** The exported script isn't a hardcoded mess; it's clean, modular Pine code that matches the Rust logic exactly.