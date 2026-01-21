//! Conversion from trendlab-yolo types to export artifacts.

use crate::artifact::{
    BacktestMetrics, BacktestResults, ComponentArtifact, ParamArtifact, StrategyArtifact,
};
use trendlab_core::ParamValue;
use trendlab_yolo::{ComponentConfig, Genome, LeaderboardEntry, RobustnessScore};

/// Convert a Genome to a StrategyArtifact.
pub fn genome_to_artifact(genome: &Genome) -> StrategyArtifact {
    let mut artifact = StrategyArtifact::new(
        genome.fingerprint(),
        component_to_artifact(&genome.signal_generator),
        component_to_artifact(&genome.position_manager),
        component_to_artifact(&genome.execution_model),
    );

    if let Some(ref filter) = genome.signal_filter {
        artifact = artifact.with_filter(component_to_artifact(filter));
    }

    artifact = artifact.with_fingerprint(genome.fingerprint());

    artifact
}

/// Convert a LeaderboardEntry to a StrategyArtifact with metrics.
pub fn entry_to_artifact(entry: &LeaderboardEntry) -> StrategyArtifact {
    let mut artifact = genome_to_artifact(&entry.genome);

    // Add robustness metrics as backtest results
    let metrics = robustness_to_metrics(&entry.robustness);
    artifact = artifact.with_results(BacktestResults {
        symbol: None,
        date_range: None,
        metrics,
    });

    artifact
}

/// Convert a ComponentConfig to ComponentArtifact.
pub fn component_to_artifact(config: &ComponentConfig) -> ComponentArtifact {
    let mut artifact = ComponentArtifact::new(config.id.0.clone());

    for (name, value) in &config.params {
        let param = param_value_to_artifact(value);
        artifact = artifact.param(name.clone(), param);
    }

    artifact
}

/// Convert a ParamValue to ParamArtifact.
pub fn param_value_to_artifact(value: &ParamValue) -> ParamArtifact {
    match value {
        ParamValue::Int(v) => ParamArtifact::Int(*v),
        ParamValue::Float(v) => ParamArtifact::Float(*v),
        ParamValue::Bool(v) => ParamArtifact::Bool(*v),
        ParamValue::Choice(v) => ParamArtifact::String(v.clone()),
    }
}

/// Convert RobustnessScore to BacktestMetrics.
pub fn robustness_to_metrics(robustness: &RobustnessScore) -> BacktestMetrics {
    BacktestMetrics {
        sharpe: Some(robustness.median_sharpe),
        max_drawdown: Some(robustness.worst_drawdown),
        win_rate: Some(robustness.avg_hit_rate),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trendlab_yolo::ComponentConfig;

    #[test]
    fn test_component_conversion() {
        let config = ComponentConfig::new("DonchianBreakout")
            .param("lookback", ParamValue::Int(20))
            .param("long_only", ParamValue::Bool(false));

        let artifact = component_to_artifact(&config);

        assert_eq!(artifact.name, "DonchianBreakout");
        assert_eq!(artifact.get_int("lookback").unwrap(), 20);
        assert!(!artifact.get_bool("long_only").unwrap());
    }

    #[test]
    fn test_genome_conversion() {
        let genome = Genome::new(
            ComponentConfig::new("DonchianBreakout"),
            ComponentConfig::new("AtrTrailingStop"),
            ComponentConfig::new("NextOpenFill"),
            None,
        );

        let artifact = genome_to_artifact(&genome);

        assert_eq!(artifact.signal_generator.name, "DonchianBreakout");
        assert_eq!(artifact.position_manager.name, "AtrTrailingStop");
        assert!(artifact.signal_filter.is_none());
    }

    #[test]
    fn test_genome_with_filter() {
        let genome = Genome::new(
            ComponentConfig::new("MaCrossover"),
            ComponentConfig::new("PercentTrailing"),
            ComponentConfig::new("NextOpenFill"),
            Some(ComponentConfig::new("AdxFilter")),
        );

        let artifact = genome_to_artifact(&genome);

        assert!(artifact.signal_filter.is_some());
        assert_eq!(artifact.signal_filter.unwrap().name, "AdxFilter");
    }
}
