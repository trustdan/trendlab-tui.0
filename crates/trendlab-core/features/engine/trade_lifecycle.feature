Feature: Trade Lifecycle
  The backtest engine must process trades through a strict lifecycle:
  Signal -> Order -> Fill, with exits checked before entries.

  Background:
    Given a backtest configuration with initial capital 100000

  @entry
  Scenario: Long entry on Donchian breakout
    Given a Donchian breakout signal generator with 20-bar lookback
    And an ATR trailing stop position manager with 2.0 multiplier
    And a next-open execution model with 10 bps slippage
    And price data where bar 21 closes above the 20-bar high
    When the backtest runs
    Then a long position should be opened
    And the entry price should include slippage

  @entry
  Scenario: Short entry on Donchian breakdown
    Given a Donchian breakout signal generator with 20-bar lookback allowing shorts
    And an ATR trailing stop position manager with 2.0 multiplier
    And a next-open execution model with 10 bps slippage
    And price data where bar 21 closes below the 20-bar low
    When the backtest runs
    Then a short position should be opened
    And the entry price should include slippage

  @exit
  Scenario: Exit on trailing stop hit
    Given a long position entered at 100.0
    And an ATR trailing stop with the stop at 95.0
    And the next bar has a low of 94.0
    When the bar is processed
    Then the position should be closed
    And the exit reason should be StopHit

  @exit-before-entry
  Scenario: Exits are checked before entries on each bar
    Given an open long position
    And a bar that would trigger both an exit and a new entry signal
    When the bar is processed
    Then the exit should occur first
    And no new entry should happen on the same bar

  @trailing-stop
  Scenario: Trailing stop ratchets in favorable direction only
    Given a long position with entry at 100.0
    And an ATR trailing stop at 95.0
    When the high since entry reaches 110.0
    Then the stop should ratchet up
    And when the price retreats to 105.0
    Then the stop should not move down

  @no-stickiness
  Scenario: Position tracking starts from entry not historical data
    Given a Donchian breakout signal with a high_since_entry reference
    When a long position is opened at 100.0
    Then high_since_entry should equal 100.0 not the historical high
    And the trailing stop should be calculated from 100.0
