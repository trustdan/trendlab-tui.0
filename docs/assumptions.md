# TrendLab Assumptions

This document records all assumptions that affect backtest correctness. When any assumption changes, you MUST:

1. Update this document
2. Add or update BDD scenarios that enforce the assumption
3. Review all code that depends on the assumption

---

## Data Assumptions

### Price Data

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Price type | **Adjusted close** | Accounts for splits and dividends; comparable across time |
| OHLC basis | Adjusted | All OHLC values are split-adjusted |
| Volume | Unadjusted | Raw share volume (not dollar volume) |

### Time

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Timezone | **UTC** | All timestamps stored in UTC |
| Bar resolution | **Daily** | End-of-day bars only (no intraday) |
| Bar timestamp | **Close time** | Timestamp represents when bar closed |
| Trading calendar | **NYSE** | Weekdays minus US market holidays |

### Missing Data

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Missing bars | **Forward-fill** | Use previous bar's close for gaps |
| Delisted symbols | **Exclude** | Remove from universe at delist date |
| Halted trading | **Skip bar** | No signal generation during halts |

---

## Fill Assumptions

### Default Execution Model

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Signal timing | **On bar close** | Signal computed after bar closes |
| Fill timing | **Next bar open** | Orders execute at next day's open |
| Fill price | **Open price** | Market order filled at open |
| Partial fills | **None** | Full fill or no fill |

### Stop Orders

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Stop check | **Intrabar** | Check if bar's low/high breached stop |
| Gap through stop | **Fill at open** | If gap through stop, fill at open price |
| Stop priority | **Before entry** | Check exits before new entries |

### Costs

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Commission | **$0** (configurable) | Default zero for research; override for realism |
| Slippage | **0 bps** (configurable) | Default zero; configurable per ExecutionModel |
| Spread | **Ignored** | Assume fills at quoted price |

---

## Position Assumptions

### Sizing

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Default size | **Equal weight** | 1/N of equity per position |
| Max positions | **Unlimited** | No hard cap (configurable) |
| Leverage | **1x** | No leverage by default |
| Fractional shares | **Allowed** | For simplicity; can constrain later |

### Direction

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Long only | **Default** | Short selling as separate component |
| Rehypothecation | **No** | No borrowing against positions |

---

## Indicator Assumptions

### Warmup

| Assumption | Value | Rationale |
|------------|-------|-----------|
| Warmup handling | **Skip signals** | No signals until warmup complete |
| Warmup bars | **Indicator-specific** | Each indicator declares its warmup |

### Calculation

| Assumption | Value | Rationale |
|------------|-------|-----------|
| ATR period | **14** (default) | Standard ATR lookback |
| SMA vs EMA | **Explicit** | Always specify which moving average |
| Lookback anchor | **Inclusive** | `n` period lookback includes current bar |

---

## Changelog

| Date | Change | Author |
|------|--------|--------|
| 2024-01-21 | Initial assumptions document | Claude |

---

## BDD Scenarios Enforcing Assumptions

Each assumption above should have corresponding BDD scenarios in `features/`. Key scenarios:

- `features/fills/next_bar_open.feature` - Verify fill timing
- `features/fills/gap_handling.feature` - Verify gap-through-stop behavior
- `features/data/adjusted_prices.feature` - Verify split adjustment
- `features/indicators/warmup.feature` - Verify no signals during warmup
