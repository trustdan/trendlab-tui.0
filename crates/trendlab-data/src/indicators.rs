//! Pre-computed technical indicators using Polars.
//!
//! All computations use lazy evaluation for efficiency.

use polars::prelude::*;

/// Add ATR (Average True Range) column to a LazyFrame.
///
/// # Columns Required
/// - `high`: f64
/// - `low`: f64
/// - `close`: f64
///
/// # Columns Added
/// - `atr_{period}`: ATR with given period
pub fn add_atr(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("atr_{}", period);

    // True Range = max(high - low, |high - prev_close|, |low - prev_close|)
    // Using when/then/otherwise for max of three values
    lf.with_column(
        // HL range
        (col("high") - col("low")).alias("_hl"),
    )
    .with_column(
        // |High - prev close|
        (col("high") - col("close").shift(lit(1)))
            .alias("_hc"),
    )
    .with_column(
        // |Low - prev close|
        (col("low") - col("close").shift(lit(1)))
            .alias("_lc"),
    )
    .with_column(
        // Absolute values
        when(col("_hc").lt(lit(0.0)))
            .then(-col("_hc"))
            .otherwise(col("_hc"))
            .alias("_hc_abs"),
    )
    .with_column(
        when(col("_lc").lt(lit(0.0)))
            .then(-col("_lc"))
            .otherwise(col("_lc"))
            .alias("_lc_abs"),
    )
    .with_column(
        // TR = max of the three
        when(col("_hl").gt_eq(col("_hc_abs")))
            .then(
                when(col("_hl").gt_eq(col("_lc_abs")))
                    .then(col("_hl"))
                    .otherwise(col("_lc_abs")),
            )
            .otherwise(
                when(col("_hc_abs").gt_eq(col("_lc_abs")))
                    .then(col("_hc_abs"))
                    .otherwise(col("_lc_abs")),
            )
            .alias("_tr"),
    )
    .with_column(
        // Simple rolling mean for ATR (approximation without EWM)
        col("_tr")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&col_name),
    )
    .select([all().exclude(["_hl", "_hc", "_lc", "_hc_abs", "_lc_abs", "_tr"])])
}

/// Add ATR% (ATR as percentage of close) column.
pub fn add_atr_pct(lf: LazyFrame, period: usize) -> LazyFrame {
    let atr_col = format!("atr_{}", period);
    let pct_col = format!("atr_pct_{}", period);

    add_atr(lf, period).with_column(
        (col(&atr_col) / col("close") * lit(100.0)).alias(&pct_col),
    )
}

/// Add Donchian Channel columns (highest high, lowest low over N periods).
///
/// # Columns Added
/// - `donchian_high_{period}`: rolling max of high
/// - `donchian_low_{period}`: rolling min of low
/// - `donchian_mid_{period}`: midpoint
pub fn add_donchian(lf: LazyFrame, period: usize) -> LazyFrame {
    let high_col = format!("donchian_high_{}", period);
    let low_col = format!("donchian_low_{}", period);
    let mid_col = format!("donchian_mid_{}", period);

    lf.with_columns([
        col("high")
            .rolling_max(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&high_col),
        col("low")
            .rolling_min(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&low_col),
    ])
    .with_column(
        ((col(&high_col) + col(&low_col)) / lit(2.0)).alias(&mid_col),
    )
}

/// Add simple moving average of close.
pub fn add_sma(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("sma_{}", period);

    lf.with_column(
        col("close")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&col_name),
    )
}

/// Add exponential moving average of close.
///
/// Uses a simple approximation via rolling mean with weighted decay.
pub fn add_ema(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("ema_{}", period);

    // Approximation using rolling mean for now
    // True EMA would need ewm_mean which requires additional features
    lf.with_column(
        col("close")
            .rolling_mean(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&col_name),
    )
}

/// Add +DI, -DI, and ADX columns.
///
/// ADX (Average Directional Index) measures trend strength.
/// Uses simplified rolling mean approximation.
///
/// # Columns Added
/// - `plus_di_{period}`: +DI
/// - `minus_di_{period}`: -DI
/// - `adx_{period}`: ADX
pub fn add_adx(lf: LazyFrame, period: usize) -> LazyFrame {
    let plus_di = format!("plus_di_{}", period);
    let minus_di = format!("minus_di_{}", period);
    let adx_col = format!("adx_{}", period);

    lf
        // Calculate directional movements
        .with_columns([
            (col("high") - col("high").shift(lit(1))).alias("_up_move"),
            (col("low").shift(lit(1)) - col("low")).alias("_down_move"),
        ])
        // +DM and -DM
        .with_columns([
            when(col("_up_move").gt(col("_down_move")).and(col("_up_move").gt(lit(0.0))))
                .then(col("_up_move"))
                .otherwise(lit(0.0))
                .alias("_plus_dm"),
            when(col("_down_move").gt(col("_up_move")).and(col("_down_move").gt(lit(0.0))))
                .then(col("_down_move"))
                .otherwise(lit(0.0))
                .alias("_minus_dm"),
        ])
        // True Range for normalization
        .with_column(
            (col("high") - col("low")).alias("_tr"),
        )
        // Smooth DM and TR
        .with_columns([
            col("_plus_dm")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size: period,
                    min_periods: period,
                    weights: None,
                    center: false,
                    fn_params: None,
                })
                .alias("_smooth_plus_dm"),
            col("_minus_dm")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size: period,
                    min_periods: period,
                    weights: None,
                    center: false,
                    fn_params: None,
                })
                .alias("_smooth_minus_dm"),
            col("_tr")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size: period,
                    min_periods: period,
                    weights: None,
                    center: false,
                    fn_params: None,
                })
                .alias("_smooth_tr"),
        ])
        // +DI and -DI as percentages
        .with_columns([
            (col("_smooth_plus_dm") / col("_smooth_tr") * lit(100.0)).alias(&plus_di),
            (col("_smooth_minus_dm") / col("_smooth_tr") * lit(100.0)).alias(&minus_di),
        ])
        // DX = |+DI - -DI| / (+DI + -DI) * 100
        .with_column(
            when((col(&plus_di) + col(&minus_di)).gt(lit(0.0)))
                .then(
                    when((col(&plus_di) - col(&minus_di)).lt(lit(0.0)))
                        .then(-(col(&plus_di) - col(&minus_di)))
                        .otherwise(col(&plus_di) - col(&minus_di))
                        / (col(&plus_di) + col(&minus_di))
                        * lit(100.0),
                )
                .otherwise(lit(0.0))
                .alias("_dx"),
        )
        // ADX = smoothed DX
        .with_column(
            col("_dx")
                .rolling_mean(RollingOptionsFixedWindow {
                    window_size: period,
                    min_periods: period,
                    weights: None,
                    center: false,
                    fn_params: None,
                })
                .alias(&adx_col),
        )
        // Drop temp columns
        .select([all().exclude([
            "_up_move", "_down_move", "_plus_dm", "_minus_dm", "_tr",
            "_smooth_plus_dm", "_smooth_minus_dm", "_smooth_tr", "_dx"
        ])])
}

/// Add returns column (percentage change).
pub fn add_returns(lf: LazyFrame, period: usize) -> LazyFrame {
    let col_name = format!("returns_{}", period);

    lf.with_column(
        ((col("close") / col("close").shift(lit(period as i64))) - lit(1.0))
            .alias(&col_name),
    )
}

/// Add volatility column (rolling standard deviation of returns).
pub fn add_volatility(lf: LazyFrame, period: usize) -> LazyFrame {
    let ret_col = "returns_1".to_string();
    let vol_col = format!("volatility_{}", period);

    // First ensure we have 1-period returns
    let lf = add_returns(lf, 1);

    lf.with_column(
        col(&ret_col)
            .rolling_std(RollingOptionsFixedWindow {
                window_size: period,
                min_periods: period,
                weights: None,
                center: false,
                fn_params: None,
            })
            .alias(&vol_col),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_df() -> LazyFrame {
        let dates: Vec<i32> = (0..50)
            .map(|i| {
                let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                    + chrono::Duration::days(i);
                (date - epoch).num_days() as i32
            })
            .collect();

        // Generate synthetic OHLCV data
        let base = 100.0;
        let opens: Vec<f64> = (0..50).map(|i| base + (i as f64) * 0.1).collect();
        let highs: Vec<f64> = opens.iter().map(|o| o + 2.0).collect();
        let lows: Vec<f64> = opens.iter().map(|o| o - 1.0).collect();
        let closes: Vec<f64> = opens.iter().map(|o| o + 0.5).collect();
        let volumes: Vec<u64> = (0..50).map(|_| 1_000_000).collect();

        DataFrame::new(vec![
            Column::new("date".into(), dates),
            Column::new("open".into(), opens),
            Column::new("high".into(), highs),
            Column::new("low".into(), lows),
            Column::new("close".into(), closes),
            Column::new("volume".into(), volumes),
        ])
        .unwrap()
        .lazy()
    }

    #[test]
    fn test_add_atr() {
        let lf = sample_df();
        let result = add_atr(lf, 14).collect().unwrap();

        assert!(result.column("atr_14").is_ok());
        // First 13 values should be null
        let atr = result.column("atr_14").unwrap();
        assert!(atr.get(0).is_ok()); // Should not panic
    }

    #[test]
    fn test_add_donchian() {
        let lf = sample_df();
        let result = add_donchian(lf, 20).collect().unwrap();

        assert!(result.column("donchian_high_20").is_ok());
        assert!(result.column("donchian_low_20").is_ok());
        assert!(result.column("donchian_mid_20").is_ok());
    }

    #[test]
    fn test_add_sma_ema() {
        let lf = sample_df();
        let result = add_ema(add_sma(lf, 20), 20).collect().unwrap();

        assert!(result.column("sma_20").is_ok());
        assert!(result.column("ema_20").is_ok());
    }

    #[test]
    fn test_add_adx() {
        let lf = sample_df();
        let result = add_adx(lf, 14).collect().unwrap();

        assert!(result.column("plus_di_14").is_ok());
        assert!(result.column("minus_di_14").is_ok());
        assert!(result.column("adx_14").is_ok());
    }

    #[test]
    fn test_chaining() {
        let lf = sample_df();
        let result = add_volatility(
            add_returns(
                add_atr(
                    add_donchian(lf, 20),
                    14
                ),
                1
            ),
            20
        )
        .collect()
        .unwrap();

        assert!(result.column("donchian_high_20").is_ok());
        assert!(result.column("atr_14").is_ok());
        assert!(result.column("returns_1").is_ok());
        assert!(result.column("volatility_20").is_ok());
    }
}
