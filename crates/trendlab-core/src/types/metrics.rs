//! Aggregate performance metrics.

use super::Trade;
use serde::{Deserialize, Serialize};

/// Aggregate performance metrics for a backtest run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    /// Total return as decimal
    pub total_return: f64,

    /// Compound Annual Growth Rate
    pub cagr: f64,

    /// Sharpe ratio (annualized)
    pub sharpe: f64,

    /// Sortino ratio (annualized)
    pub sortino: f64,

    /// Maximum drawdown as decimal (negative)
    pub max_drawdown: f64,

    /// Win rate as decimal
    pub win_rate: f64,

    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,

    /// Total number of trades
    pub total_trades: usize,

    /// Average bars held per trade
    pub avg_bars_held: f64,

    /// Average trade return
    pub avg_return: f64,

    /// Average winner return
    pub avg_winner: f64,

    /// Average loser return
    pub avg_loser: f64,

    /// Largest winning trade
    pub best_trade: f64,

    /// Largest losing trade
    pub worst_trade: f64,

    /// Maximum consecutive winners
    pub max_consecutive_wins: usize,

    /// Maximum consecutive losers
    pub max_consecutive_losses: usize,
}

impl Metrics {
    /// Calculate metrics from trades and equity curve.
    ///
    /// # Arguments
    /// - `trades`: Completed trades
    /// - `equity_curve`: Daily equity values
    /// - `trading_days_per_year`: Typically 252
    pub fn calculate(
        trades: &[Trade],
        equity_curve: &[f64],
        trading_days_per_year: f64,
    ) -> Self {
        if trades.is_empty() || equity_curve.len() < 2 {
            return Self::default();
        }

        let initial = equity_curve[0];
        let final_eq = *equity_curve.last().unwrap();
        let total_return = (final_eq / initial) - 1.0;

        let years = equity_curve.len() as f64 / trading_days_per_year;
        let cagr = if years > 0.0 {
            (final_eq / initial).powf(1.0 / years) - 1.0
        } else {
            0.0
        };

        // Daily returns for Sharpe/Sortino
        let daily_returns: Vec<f64> = equity_curve
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();

        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;

        let variance = daily_returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / daily_returns.len() as f64;
        let std_dev = variance.sqrt();

        let sharpe = if std_dev > 0.0 {
            (mean_return / std_dev) * trading_days_per_year.sqrt()
        } else {
            0.0
        };

        // Sortino (downside deviation)
        let downside: Vec<f64> = daily_returns.iter().filter(|&&r| r < 0.0).copied().collect();
        let downside_std = if !downside.is_empty() {
            (downside.iter().map(|r| r.powi(2)).sum::<f64>() / downside.len() as f64).sqrt()
        } else {
            0.0
        };
        let sortino = if downside_std > 0.0 {
            (mean_return / downside_std) * trading_days_per_year.sqrt()
        } else {
            0.0
        };

        // Maximum drawdown
        let max_drawdown = Self::calculate_max_drawdown(equity_curve);

        // Trade statistics
        let winners: Vec<&Trade> = trades.iter().filter(|t| t.is_winner()).collect();
        let losers: Vec<&Trade> = trades.iter().filter(|t| t.is_loser()).collect();

        let win_rate = winners.len() as f64 / trades.len() as f64;

        let gross_profit: f64 = winners.iter().map(|t| t.return_pct).sum();
        let gross_loss: f64 = losers.iter().map(|t| t.return_pct.abs()).sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let avg_return = trades.iter().map(|t| t.return_pct).sum::<f64>() / trades.len() as f64;
        let avg_bars_held =
            trades.iter().map(|t| t.bars_held).sum::<usize>() as f64 / trades.len() as f64;

        let avg_winner = if !winners.is_empty() {
            winners.iter().map(|t| t.return_pct).sum::<f64>() / winners.len() as f64
        } else {
            0.0
        };

        let avg_loser = if !losers.is_empty() {
            losers.iter().map(|t| t.return_pct).sum::<f64>() / losers.len() as f64
        } else {
            0.0
        };

        let best_trade = trades
            .iter()
            .map(|t| t.return_pct)
            .fold(f64::NEG_INFINITY, f64::max);

        let worst_trade = trades
            .iter()
            .map(|t| t.return_pct)
            .fold(f64::INFINITY, f64::min);

        let (max_consecutive_wins, max_consecutive_losses) =
            Self::calculate_consecutive_streaks(trades);

        Self {
            total_return,
            cagr,
            sharpe,
            sortino,
            max_drawdown,
            win_rate,
            profit_factor,
            total_trades: trades.len(),
            avg_bars_held,
            avg_return,
            avg_winner,
            avg_loser,
            best_trade,
            worst_trade,
            max_consecutive_wins,
            max_consecutive_losses,
        }
    }

    /// Calculate maximum drawdown from equity curve.
    fn calculate_max_drawdown(equity_curve: &[f64]) -> f64 {
        let mut max_equity = equity_curve[0];
        let mut max_dd: f64 = 0.0;

        for &equity in equity_curve {
            max_equity = max_equity.max(equity);
            let dd = (equity - max_equity) / max_equity;
            max_dd = max_dd.min(dd);
        }

        max_dd
    }

    /// Calculate max consecutive wins and losses.
    fn calculate_consecutive_streaks(trades: &[Trade]) -> (usize, usize) {
        let mut max_wins = 0;
        let mut max_losses = 0;
        let mut current_wins = 0;
        let mut current_losses = 0;

        for trade in trades {
            if trade.is_winner() {
                current_wins += 1;
                current_losses = 0;
                max_wins = max_wins.max(current_wins);
            } else if trade.is_loser() {
                current_losses += 1;
                current_wins = 0;
                max_losses = max_losses.max(current_losses);
            } else {
                // Break-even trade breaks both streaks
                current_wins = 0;
                current_losses = 0;
            }
        }

        (max_wins, max_losses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Direction, ExitReason};
    use chrono::NaiveDate;

    fn make_trade(return_pct: f64, bars_held: usize) -> Trade {
        let entry_price = 100.0;
        let exit_price = entry_price * (1.0 + return_pct);

        Trade {
            entry_bar_idx: 0,
            entry_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            entry_price,
            exit_bar_idx: bars_held,
            exit_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            exit_price,
            direction: Direction::Long,
            size: 10000.0,
            exit_reason: ExitReason::StopHit,
            return_pct,
            bars_held,
            mae: -0.02,
            mfe: 0.05,
        }
    }

    #[test]
    fn test_basic_metrics() {
        let trades = vec![
            make_trade(0.10, 5),
            make_trade(-0.05, 3),
            make_trade(0.08, 4),
        ];

        // Simple equity curve: 100 -> 110 -> 104.5 -> 112.86
        let equity = vec![100.0, 110.0, 104.5, 112.86];

        let metrics = Metrics::calculate(&trades, &equity, 252.0);

        assert_eq!(metrics.total_trades, 3);
        assert!((metrics.win_rate - (2.0 / 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_max_drawdown() {
        let equity = vec![100.0, 110.0, 105.0, 115.0, 100.0, 120.0];
        let dd = Metrics::calculate_max_drawdown(&equity);
        // Max DD: from 115 to 100 = -13.04%
        assert!((dd - (-15.0 / 115.0)).abs() < 1e-10);
    }

    #[test]
    fn test_consecutive_streaks() {
        let trades = vec![
            make_trade(0.05, 5),
            make_trade(0.03, 3),
            make_trade(0.07, 4),
            make_trade(-0.02, 2),
            make_trade(-0.03, 3),
        ];

        let (wins, losses) = Metrics::calculate_consecutive_streaks(&trades);
        assert_eq!(wins, 3);
        assert_eq!(losses, 2);
    }

    #[test]
    fn test_empty_trades() {
        let metrics = Metrics::calculate(&[], &[100.0], 252.0);
        assert_eq!(metrics.total_trades, 0);
        assert_eq!(metrics.sharpe, 0.0);
    }
}
