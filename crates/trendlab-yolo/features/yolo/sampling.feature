Feature: YOLO Structural Monte Carlo Sampling
  The YOLO sampler explores the strategy space by sampling structure first,
  then parameters. This ensures fair comparison across component types.

  Background:
    Given a component registry with default components

  @warmup
  Scenario: Warmup phase uses uniform sampling
    Given a YOLO session with 10 warmup iterations
    When the session starts
    Then the phase should be Warmup
    And samples should be drawn uniformly from the structural space

  @exploitation
  Scenario: Exploitation phase biases towards high performers
    Given a YOLO session with 5 warmup iterations and 20 total iterations
    And the session has completed warmup with scores ranging 0.2 to 0.8
    When exploitation sampling begins
    Then higher-scoring genomes should have higher selection probability

  @structural-mutation
  Scenario: Structural mutation swaps one component type
    Given a genome with signal generator "donchian_breakout"
    When a structural mutation is applied
    Then exactly one component type should change
    And the parameters for the new component should be freshly sampled

  @param-mutation
  Scenario: Parameter mutation preserves structure
    Given a genome with signal generator "donchian_breakout"
    And position manager "atr_trailing_stop"
    When a parameter mutation is applied
    Then the component IDs should remain unchanged
    And at least one parameter value may differ

  @determinism
  Scenario: Same seed produces identical sampling sequence
    Given two samplers initialized with seed 42
    When each samples 10 genomes
    Then the genome sequences should be identical

  @component-coverage
  Scenario: All component types are reachable
    Given 100 uniform samples
    Then every registered component ID should appear at least once

  @leaderboard-invariant-e
  Scenario: Three leaderboards must exist before trusting overall
    Given a YOLO session that has only submitted to Overall leaderboard
    Then invariant E should not be satisfied
    When entries are submitted to SignalQuality, PositionManagement, and ExecutionSensitivity
    Then invariant E should be satisfied

  @robustness-scoring
  Scenario: Low trade count invalidates strategy
    Given a backtest result with Sharpe 2.0 but only 5 trades
    When scored by the robustness scorer
    Then the result should be marked invalid
    And it should not appear on any leaderboard

  @robustness-scoring
  Scenario: High drawdown penalizes score
    Given a backtest result with Sharpe 1.5 and max drawdown 30%
    When scored by the robustness scorer
    Then the score should be lower than a similar result with 10% drawdown

  @attribution
  Scenario: Component attribution identifies outperformers
    Given strategies using signal generator "sg_good" average score 0.8
    And strategies using signal generator "sg_bad" average score 0.3
    When component attribution is computed
    Then "sg_good" should have positive marginal contribution
    And "sg_bad" should have negative marginal contribution
