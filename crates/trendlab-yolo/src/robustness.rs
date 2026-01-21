//! Robustness scoring for strategy evaluation.
//!
//! The robustness scorer evaluates strategies not just by raw returns but by
//! consistency, drawdown resilience, and sensitivity to execution assumptions.
//!
//! # Philosophy
//!
//! A strategy that performs well on average but has high variance or sensitivity
//! to execution costs is less trustworthy than one with slightly lower returns
//! but consistent behavior across conditions.
//!
//! # Scoring Formula
//!
//! ```text
//! base = w_median * median_sharpe + w_hit * hit_rate + w_consistency * -std + w_floor * floor
//! robustness = base * (1 - dd_penalty * max_dd) * (1 - fragility * cost_sens)
//! ```

use serde::{Deserialize, Serialize};
use trendlab_core::Metrics;

/// Configuration for robustness scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessConfig {
    /// Weight for median Sharpe ratio (default: 0.4)
    pub weight_median_sharpe: f64,
    /// Weight for hit rate (win rate) (default: 0.1)
    pub weight_hit_rate: f64,
    /// Weight for consistency (-std of returns) (default: 0.2)
    pub weight_consistency: f64,
    /// Weight for floor (worst-case return) (default: 0.3)
    pub weight_floor: f64,

    /// Penalty multiplier for maximum drawdown (default: 0.5)
    pub drawdown_penalty: f64,
    /// Penalty multiplier for fragility (cost sensitivity) (default: 0.3)
    pub fragility_penalty: f64,

    /// Minimum trades required for valid score (default: 10)
    pub min_trades: usize,
    /// Minimum Sharpe for consideration (default: -0.5)
    pub min_sharpe: f64,
}

impl Default for RobustnessConfig {
    fn default() -> Self {
        Self {
            weight_median_sharpe: 0.4,
            weight_hit_rate: 0.1,
            weight_consistency: 0.2,
            weight_floor: 0.3,
            drawdown_penalty: 0.5,
            fragility_penalty: 0.3,
            min_trades: 10,
            min_sharpe: -0.5,
        }
    }
}

/// Input data for robustness calculation.
///
/// Contains results from multiple runs across different conditions:
/// - Different symbols
/// - Different time periods
/// - Different execution assumptions
#[derive(Debug, Clone, Default)]
pub struct RobustnessInput {
    /// Sharpe ratios from multiple runs
    pub sharpe_values: Vec<f64>,
    /// Win rates from multiple runs
    pub hit_rates: Vec<f64>,
    /// Total returns from multiple runs
    pub returns: Vec<f64>,
    /// Maximum drawdowns from multiple runs (negative values)
    pub max_drawdowns: Vec<f64>,
    /// Trade counts from multiple runs
    pub trade_counts: Vec<usize>,
    /// Cost sensitivity (how much does performance degrade with higher costs)
    pub cost_sensitivity: Option<f64>,
}

impl RobustnessInput {
    /// Create input from a single backtest result.
    pub fn from_metrics(metrics: &Metrics) -> Self {
        Self {
            sharpe_values: vec![metrics.sharpe],
            hit_rates: vec![metrics.win_rate],
            returns: vec![metrics.total_return],
            max_drawdowns: vec![metrics.max_drawdown],
            trade_counts: vec![metrics.total_trades],
            cost_sensitivity: None,
        }
    }

    /// Add results from another run.
    pub fn add_run(&mut self, metrics: &Metrics) {
        self.sharpe_values.push(metrics.sharpe);
        self.hit_rates.push(metrics.win_rate);
        self.returns.push(metrics.total_return);
        self.max_drawdowns.push(metrics.max_drawdown);
        self.trade_counts.push(metrics.total_trades);
    }

    /// Merge with another input.
    pub fn merge(&mut self, other: &RobustnessInput) {
        self.sharpe_values.extend(&other.sharpe_values);
        self.hit_rates.extend(&other.hit_rates);
        self.returns.extend(&other.returns);
        self.max_drawdowns.extend(&other.max_drawdowns);
        self.trade_counts.extend(&other.trade_counts);
    }

    /// Set cost sensitivity.
    pub fn with_cost_sensitivity(mut self, sensitivity: f64) -> Self {
        self.cost_sensitivity = Some(sensitivity);
        self
    }
}

/// Robustness score result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustnessScore {
    /// Final robustness score (higher is better)
    pub score: f64,
    /// Base score before penalties
    pub base_score: f64,
    /// Median Sharpe contribution
    pub median_sharpe: f64,
    /// Hit rate contribution
    pub avg_hit_rate: f64,
    /// Consistency (negative std)
    pub consistency: f64,
    /// Floor (worst return)
    pub floor: f64,
    /// Maximum drawdown (worst case)
    pub worst_drawdown: f64,
    /// Cost sensitivity factor
    pub cost_sensitivity: f64,
    /// Number of runs used in calculation
    pub num_runs: usize,
    /// Whether the score is valid (enough trades, etc.)
    pub is_valid: bool,
    /// Reason if invalid
    pub invalid_reason: Option<String>,
}

impl Default for RobustnessScore {
    fn default() -> Self {
        Self {
            score: 0.0,
            base_score: 0.0,
            median_sharpe: 0.0,
            avg_hit_rate: 0.0,
            consistency: 0.0,
            floor: 0.0,
            worst_drawdown: 0.0,
            cost_sensitivity: 0.0,
            num_runs: 0,
            is_valid: false,
            invalid_reason: Some("No data".to_string()),
        }
    }
}

/// Robustness scorer.
///
/// Evaluates strategies using a multi-factor scoring formula that penalizes
/// drawdowns and cost sensitivity.
#[derive(Debug, Clone)]
pub struct RobustnessScorer {
    config: RobustnessConfig,
}

impl RobustnessScorer {
    /// Create a new scorer with the given configuration.
    pub fn new(config: RobustnessConfig) -> Self {
        Self { config }
    }

    /// Create a scorer with default configuration.
    pub fn default_scorer() -> Self {
        Self::new(RobustnessConfig::default())
    }

    /// Calculate robustness score from input data.
    pub fn score(&self, input: &RobustnessInput) -> RobustnessScore {
        // Check minimum requirements
        if input.sharpe_values.is_empty() {
            return RobustnessScore {
                invalid_reason: Some("No runs provided".to_string()),
                ..Default::default()
            };
        }

        let total_trades: usize = input.trade_counts.iter().sum();
        if total_trades < self.config.min_trades {
            return RobustnessScore {
                num_runs: input.sharpe_values.len(),
                invalid_reason: Some(format!(
                    "Insufficient trades: {} < {}",
                    total_trades, self.config.min_trades
                )),
                ..Default::default()
            };
        }

        // Calculate metrics
        let median_sharpe = Self::median(&input.sharpe_values);
        let avg_hit_rate = Self::mean(&input.hit_rates);
        let std_returns = Self::std_dev(&input.returns);
        let floor = input
            .returns
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let worst_drawdown = input
            .max_drawdowns
            .iter()
            .cloned()
            .fold(0.0_f64, |a, b| a.min(b)); // Most negative

        // Check minimum Sharpe
        if median_sharpe < self.config.min_sharpe {
            return RobustnessScore {
                num_runs: input.sharpe_values.len(),
                median_sharpe,
                invalid_reason: Some(format!(
                    "Sharpe too low: {:.2} < {:.2}",
                    median_sharpe, self.config.min_sharpe
                )),
                ..Default::default()
            };
        }

        // Normalize metrics for scoring
        // Sharpe: typically -1 to 3, normalize to 0-1 range
        let sharpe_norm = ((median_sharpe + 1.0) / 4.0).clamp(0.0, 1.0);

        // Hit rate: already 0-1
        let hit_rate_norm = avg_hit_rate;

        // Consistency: std 0-0.5 -> 1-0 (lower std = higher consistency)
        let consistency = (1.0 - std_returns * 2.0).clamp(0.0, 1.0);

        // Floor: -1 to 1 -> 0 to 1
        let floor_norm = ((floor + 1.0) / 2.0).clamp(0.0, 1.0);

        // Calculate base score
        let base_score = self.config.weight_median_sharpe * sharpe_norm
            + self.config.weight_hit_rate * hit_rate_norm
            + self.config.weight_consistency * consistency
            + self.config.weight_floor * floor_norm;

        // Apply penalties
        let dd_factor = 1.0 - self.config.drawdown_penalty * worst_drawdown.abs();
        let cost_sens = input.cost_sensitivity.unwrap_or(0.0);
        let fragility_factor = 1.0 - self.config.fragility_penalty * cost_sens;

        let score = base_score * dd_factor.max(0.0) * fragility_factor.max(0.0);

        RobustnessScore {
            score,
            base_score,
            median_sharpe,
            avg_hit_rate,
            consistency,
            floor,
            worst_drawdown,
            cost_sensitivity: cost_sens,
            num_runs: input.sharpe_values.len(),
            is_valid: true,
            invalid_reason: None,
        }
    }

    /// Calculate cost sensitivity.
    ///
    /// Runs the same strategy with different cost assumptions and measures
    /// how much performance degrades.
    ///
    /// Returns a value 0-1 where 0 = no sensitivity, 1 = highly sensitive.
    pub fn calculate_cost_sensitivity(
        base_sharpe: f64,
        high_cost_sharpe: f64,
        cost_multiplier: f64,
    ) -> f64 {
        if base_sharpe <= 0.0 {
            return 1.0; // Already bad, fully sensitive
        }

        let sharpe_drop = base_sharpe - high_cost_sharpe;
        let relative_drop = sharpe_drop / base_sharpe;

        // Normalize by cost multiplier (e.g., 2x costs should have 2x more impact)
        let sensitivity = relative_drop / (cost_multiplier - 1.0).max(0.1);

        sensitivity.clamp(0.0, 1.0)
    }

    /// Calculate median of a slice.
    fn median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mut sorted: Vec<f64> = values.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    /// Calculate mean of a slice.
    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }

    /// Calculate standard deviation of a slice.
    fn std_dev(values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let mean = Self::mean(values);
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        variance.sqrt()
    }
}

impl Default for RobustnessScorer {
    fn default() -> Self {
        Self::default_scorer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let scorer = RobustnessScorer::default();
        let input = RobustnessInput::default();
        let score = scorer.score(&input);

        assert!(!score.is_valid);
        assert!(score.invalid_reason.is_some());
    }

    #[test]
    fn test_insufficient_trades() {
        let scorer = RobustnessScorer::default();
        let input = RobustnessInput {
            sharpe_values: vec![1.0],
            hit_rates: vec![0.6],
            returns: vec![0.2],
            max_drawdowns: vec![-0.1],
            trade_counts: vec![5], // Less than default min_trades (10)
            cost_sensitivity: None,
        };

        let score = scorer.score(&input);
        assert!(!score.is_valid);
        assert!(score.invalid_reason.unwrap().contains("Insufficient trades"));
    }

    #[test]
    fn test_valid_score() {
        let scorer = RobustnessScorer::default();
        let input = RobustnessInput {
            sharpe_values: vec![1.5, 1.3, 1.7],
            hit_rates: vec![0.55, 0.52, 0.58],
            returns: vec![0.20, 0.18, 0.22],
            max_drawdowns: vec![-0.10, -0.12, -0.08],
            trade_counts: vec![50, 45, 55],
            cost_sensitivity: None,
        };

        let score = scorer.score(&input);

        assert!(score.is_valid);
        assert!(score.score > 0.0);
        assert!(score.score <= 1.0);
        assert_eq!(score.num_runs, 3);
    }

    #[test]
    fn test_drawdown_penalty() {
        let scorer = RobustnessScorer::default();

        let good_input = RobustnessInput {
            sharpe_values: vec![1.5],
            hit_rates: vec![0.55],
            returns: vec![0.20],
            max_drawdowns: vec![-0.05], // Small drawdown
            trade_counts: vec![50],
            cost_sensitivity: None,
        };

        let bad_input = RobustnessInput {
            sharpe_values: vec![1.5],
            hit_rates: vec![0.55],
            returns: vec![0.20],
            max_drawdowns: vec![-0.40], // Large drawdown
            trade_counts: vec![50],
            cost_sensitivity: None,
        };

        let good_score = scorer.score(&good_input);
        let bad_score = scorer.score(&bad_input);

        assert!(good_score.score > bad_score.score);
    }

    #[test]
    fn test_cost_sensitivity_penalty() {
        let scorer = RobustnessScorer::default();

        let robust_input = RobustnessInput {
            sharpe_values: vec![1.5],
            hit_rates: vec![0.55],
            returns: vec![0.20],
            max_drawdowns: vec![-0.10],
            trade_counts: vec![50],
            cost_sensitivity: Some(0.1), // Low sensitivity
        };

        let fragile_input = RobustnessInput {
            sharpe_values: vec![1.5],
            hit_rates: vec![0.55],
            returns: vec![0.20],
            max_drawdowns: vec![-0.10],
            trade_counts: vec![50],
            cost_sensitivity: Some(0.8), // High sensitivity
        };

        let robust_score = scorer.score(&robust_input);
        let fragile_score = scorer.score(&fragile_input);

        assert!(robust_score.score > fragile_score.score);
    }

    #[test]
    fn test_median_calculation() {
        assert!((RobustnessScorer::median(&[1.0, 2.0, 3.0]) - 2.0).abs() < 1e-10);
        assert!((RobustnessScorer::median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-10);
        assert!((RobustnessScorer::median(&[3.0, 1.0, 2.0]) - 2.0).abs() < 1e-10);
        assert!((RobustnessScorer::median(&[5.0]) - 5.0).abs() < 1e-10);
        assert!((RobustnessScorer::median(&[]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_cost_sensitivity_calculation() {
        // Base Sharpe 2.0, high cost Sharpe 1.5 with 2x costs
        let sens = RobustnessScorer::calculate_cost_sensitivity(2.0, 1.5, 2.0);
        // Drop of 0.5/2.0 = 0.25, normalized by (2-1) = 0.25
        assert!((sens - 0.25).abs() < 1e-10);

        // Zero base Sharpe should return 1.0
        let sens = RobustnessScorer::calculate_cost_sensitivity(0.0, -0.5, 2.0);
        assert!((sens - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_low_sharpe_rejection() {
        let scorer = RobustnessScorer::default();
        let input = RobustnessInput {
            sharpe_values: vec![-1.0], // Below min_sharpe (-0.5)
            hit_rates: vec![0.55],
            returns: vec![0.20],
            max_drawdowns: vec![-0.10],
            trade_counts: vec![50],
            cost_sensitivity: None,
        };

        let score = scorer.score(&input);
        assert!(!score.is_valid);
        assert!(score.invalid_reason.unwrap().contains("Sharpe too low"));
    }
}
