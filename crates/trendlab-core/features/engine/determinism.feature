Feature: Determinism
  The backtest engine must produce identical results given the same inputs.
  This is critical for reproducibility and debugging.

  @determinism
  Scenario: Same inputs produce identical results
    Given a fixed strategy configuration
    And a fixed price dataset of 252 bars
    When the backtest runs twice with the same inputs
    Then both runs should produce identical trade counts
    And both runs should produce identical final equity
    And both runs should produce identical trade-by-trade results

  @determinism
  Scenario: Results are independent of system state
    Given a fixed strategy configuration
    And a fixed price dataset
    When the backtest runs after various system operations
    Then the results should match the baseline run

  @no-lookahead
  Scenario: Bar N can only see bars 0 through N
    Given a signal generator that records accessed bar indices
    When the backtest processes bar 50
    Then only bars 0 through 50 should have been accessed
    And bar 51 and beyond should not be visible

  @fresh-state
  Scenario: Each backtest run starts with fresh component state
    Given a strategy with stateful components
    When the backtest runs twice in sequence
    Then the second run should not be affected by the first run
    And both runs should produce identical results

  @fingerprint
  Scenario: Run fingerprint uniquely identifies configuration
    Given two different strategy configurations
    When fingerprints are generated for each
    Then the fingerprints should be different
    And rerunning with the same config should produce the same fingerprint
