//! Component attribution for analyzing what drives performance.
//!
//! Attribution helps answer: "Which component is responsible for this strategy's performance?"
//!
//! # Methodology
//!
//! 1. **Marginal Attribution**: Compare average performance when a component is present
//!    vs when it's absent across the population.
//! 2. **Interaction Effects**: Some component combinations may perform better together
//!    than their individual contributions suggest.

use crate::genome::{ComponentId, Genome};
use crate::leaderboard::LeaderboardEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Component type for attribution analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    /// Signal generator
    SignalGenerator,
    /// Position manager
    PositionManager,
    /// Execution model
    ExecutionModel,
    /// Signal filter
    SignalFilter,
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentType::SignalGenerator => write!(f, "Signal Generator"),
            ComponentType::PositionManager => write!(f, "Position Manager"),
            ComponentType::ExecutionModel => write!(f, "Execution Model"),
            ComponentType::SignalFilter => write!(f, "Signal Filter"),
        }
    }
}

/// Attribution score for a specific component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScore {
    /// Component ID
    pub id: ComponentId,
    /// Component type
    pub component_type: ComponentType,
    /// Average robustness score when this component is used
    pub avg_score: f64,
    /// Number of strategies using this component
    pub count: usize,
    /// Best score achieved with this component
    pub best_score: f64,
    /// Marginal contribution (how much better than baseline)
    pub marginal_contribution: f64,
}

/// Attribution analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionResult {
    /// Scores by signal generator
    pub signal_generators: Vec<ComponentScore>,
    /// Scores by position manager
    pub position_managers: Vec<ComponentScore>,
    /// Scores by execution model
    pub execution_models: Vec<ComponentScore>,
    /// Scores by signal filter
    pub signal_filters: Vec<ComponentScore>,
    /// Population baseline score (average across all)
    pub baseline_score: f64,
    /// Number of strategies analyzed
    pub total_strategies: usize,
}

/// Component attribution analyzer.
///
/// Analyzes which components contribute most to strategy performance.
#[derive(Debug, Clone, Default)]
pub struct ComponentAttribution {
    /// Accumulated scores by component
    scores: HashMap<(ComponentType, String), Vec<f64>>,
    /// Best score by component
    best: HashMap<(ComponentType, String), f64>,
    /// Total samples
    total: usize,
    /// Sum of all scores for baseline
    score_sum: f64,
}

impl ComponentAttribution {
    /// Create a new attribution analyzer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a strategy result to the analysis.
    pub fn add(&mut self, genome: &Genome, score: f64) {
        self.total += 1;
        self.score_sum += score;

        // Record signal generator
        self.record(
            ComponentType::SignalGenerator,
            &genome.signal_generator.id.0,
            score,
        );

        // Record position manager
        self.record(
            ComponentType::PositionManager,
            &genome.position_manager.id.0,
            score,
        );

        // Record execution model
        self.record(
            ComponentType::ExecutionModel,
            &genome.execution_model.id.0,
            score,
        );

        // Record signal filter (if present)
        if let Some(ref filter) = genome.signal_filter {
            self.record(ComponentType::SignalFilter, &filter.id.0, score);
        } else {
            self.record(ComponentType::SignalFilter, "none", score);
        }
    }

    /// Add from a leaderboard entry.
    pub fn add_entry(&mut self, entry: &LeaderboardEntry) {
        self.add(&entry.genome, entry.robustness.score);
    }

    /// Record a component score.
    fn record(&mut self, component_type: ComponentType, id: &str, score: f64) {
        let key = (component_type, id.to_string());

        self.scores.entry(key.clone()).or_default().push(score);

        self.best
            .entry(key)
            .and_modify(|b| *b = b.max(score))
            .or_insert(score);
    }

    /// Compute attribution results.
    pub fn compute(&self) -> AttributionResult {
        let baseline = if self.total > 0 {
            self.score_sum / self.total as f64
        } else {
            0.0
        };

        let signal_generators = self.compute_scores(ComponentType::SignalGenerator, baseline);
        let position_managers = self.compute_scores(ComponentType::PositionManager, baseline);
        let execution_models = self.compute_scores(ComponentType::ExecutionModel, baseline);
        let signal_filters = self.compute_scores(ComponentType::SignalFilter, baseline);

        AttributionResult {
            signal_generators,
            position_managers,
            execution_models,
            signal_filters,
            baseline_score: baseline,
            total_strategies: self.total,
        }
    }

    /// Compute scores for a component type.
    fn compute_scores(&self, component_type: ComponentType, baseline: f64) -> Vec<ComponentScore> {
        let mut scores: Vec<ComponentScore> = self
            .scores
            .iter()
            .filter(|((ct, _), _)| *ct == component_type)
            .map(|((_, id), score_list)| {
                let avg = if score_list.is_empty() {
                    0.0
                } else {
                    score_list.iter().sum::<f64>() / score_list.len() as f64
                };

                let best = self
                    .best
                    .get(&(component_type, id.clone()))
                    .copied()
                    .unwrap_or(0.0);

                ComponentScore {
                    id: ComponentId::new(id.clone()),
                    component_type,
                    avg_score: avg,
                    count: score_list.len(),
                    best_score: best,
                    marginal_contribution: avg - baseline,
                }
            })
            .collect();

        // Sort by marginal contribution descending
        scores.sort_by(|a, b| {
            b.marginal_contribution
                .partial_cmp(&a.marginal_contribution)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scores
    }

    /// Get the top N components by marginal contribution.
    pub fn top_components(&self, n: usize) -> Vec<ComponentScore> {
        let result = self.compute();

        let mut all: Vec<ComponentScore> = Vec::new();
        all.extend(result.signal_generators);
        all.extend(result.position_managers);
        all.extend(result.execution_models);
        all.extend(result.signal_filters);

        all.sort_by(|a, b| {
            b.marginal_contribution
                .partial_cmp(&a.marginal_contribution)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        all.into_iter().take(n).collect()
    }

    /// Get components with negative marginal contribution (underperformers).
    pub fn underperformers(&self) -> Vec<ComponentScore> {
        let result = self.compute();

        let mut all: Vec<ComponentScore> = Vec::new();
        all.extend(result.signal_generators);
        all.extend(result.position_managers);
        all.extend(result.execution_models);
        all.extend(result.signal_filters);

        all.into_iter()
            .filter(|c| c.marginal_contribution < 0.0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::ComponentConfig;

    fn make_genome(sg: &str, pm: &str, em: &str, filter: Option<&str>) -> Genome {
        Genome::new(
            ComponentConfig::new(sg),
            ComponentConfig::new(pm),
            ComponentConfig::new(em),
            filter.map(ComponentConfig::new),
        )
    }

    #[test]
    fn test_attribution_basic() {
        let mut attr = ComponentAttribution::new();

        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.8);
        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.7);
        attr.add(&make_genome("sg2", "pm1", "em1", None), 0.5);

        let result = attr.compute();

        assert_eq!(result.total_strategies, 3);
        assert!((result.baseline_score - (0.8 + 0.7 + 0.5) / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_marginal_contribution() {
        let mut attr = ComponentAttribution::new();

        // sg1 is used in high-performing strategies
        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.9);
        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.8);

        // sg2 is used in low-performing strategies
        attr.add(&make_genome("sg2", "pm1", "em1", None), 0.3);
        attr.add(&make_genome("sg2", "pm1", "em1", None), 0.2);

        let result = attr.compute();

        // Find sg1 and sg2
        let sg1 = result
            .signal_generators
            .iter()
            .find(|s| s.id.0 == "sg1")
            .unwrap();
        let sg2 = result
            .signal_generators
            .iter()
            .find(|s| s.id.0 == "sg2")
            .unwrap();

        // sg1 should have positive marginal contribution
        assert!(sg1.marginal_contribution > 0.0);
        // sg2 should have negative marginal contribution
        assert!(sg2.marginal_contribution < 0.0);
    }

    #[test]
    fn test_top_components() {
        let mut attr = ComponentAttribution::new();

        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.9);
        attr.add(&make_genome("sg1", "pm2", "em1", None), 0.8);
        attr.add(&make_genome("sg2", "pm1", "em1", None), 0.3);
        attr.add(&make_genome("sg2", "pm2", "em1", None), 0.2);

        let top = attr.top_components(2);

        assert_eq!(top.len(), 2);
        // Top should have positive marginal contributions
        assert!(top[0].marginal_contribution >= top[1].marginal_contribution);
    }

    #[test]
    fn test_underperformers() {
        let mut attr = ComponentAttribution::new();

        attr.add(&make_genome("sg1", "pm1", "em1", None), 0.9);
        attr.add(&make_genome("sg2", "pm1", "em1", None), 0.2);

        let under = attr.underperformers();

        // sg2 should be an underperformer (below baseline)
        assert!(under.iter().any(|c| c.id.0 == "sg2"));
    }

    #[test]
    fn test_component_type_display() {
        assert_eq!(ComponentType::SignalGenerator.to_string(), "Signal Generator");
        assert_eq!(ComponentType::PositionManager.to_string(), "Position Manager");
        assert_eq!(
            ComponentType::ExecutionModel.to_string(),
            "Execution Model"
        );
        assert_eq!(ComponentType::SignalFilter.to_string(), "Signal Filter");
    }
}
