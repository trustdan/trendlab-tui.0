# Test Fixtures

This directory contains small, deterministic datasets for testing. **Never use live market data in tests.**

## Guidelines

1. **Keep fixtures small** - 20-200 bars maximum
2. **Make them deterministic** - No random data; same input = same output
3. **Document the scenario** - Each fixture should have a comment explaining what it tests
4. **Use CSV for readability** - Parquet for performance tests only

## Fixture Naming Convention

```text
{scenario}_{bars}bars.csv
```

Examples:

- `trending_up_50bars.csv` - Clear uptrend for signal testing
- `choppy_100bars.csv` - Sideways market for filter testing
- `gap_down_20bars.csv` - Gap scenario for execution testing
- `split_adjusted_30bars.csv` - Split-adjusted price testing

## Required Columns

All fixture CSVs must have these columns:

```text
timestamp,open,high,low,close,volume
```

Optional columns:

```text
adj_close,atr_14,adx_14
```

## Example Fixture

```csv
timestamp,open,high,low,close,volume
2024-01-02,100.00,102.00,99.00,101.50,1000000
2024-01-03,101.50,103.00,101.00,102.75,1200000
2024-01-04,102.75,105.00,102.00,104.50,1500000
```

## Do NOT

- Commit real market data (even anonymized)
- Create fixtures larger than 500 bars
- Use fixtures that depend on external files
- Modify existing fixtures without updating tests
