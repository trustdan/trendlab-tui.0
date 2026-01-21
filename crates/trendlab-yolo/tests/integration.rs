//! Integration tests for YOLO structural Monte Carlo sampling.

use trendlab_yolo::{
    attribution::ComponentAttribution,
    genome::{ComponentConfig, Genome},
    leaderboard::{LeaderboardSet, LeaderboardType},
    registry::ComponentRegistry,
    robustness::{RobustnessConfig, RobustnessInput, RobustnessScore, RobustnessScorer},
    sampler::{SamplerConfig, StructuralSampler},
    session::{SessionConfig, SessionPhase, YoloSession},
};

fn make_metrics(sharpe: f64, trades: usize) -> trendlab_core::Metrics {
    trendlab_core::Metrics {
        sharpe,
        total_trades: trades,
        win_rate: 0.55,
        total_return: 0.20,
        max_drawdown: -0.10,
        ..Default::default()
    }
}

// =============================================================================
// Session lifecycle tests
// =============================================================================

#[test]
fn test_session_warmup_phase() {
    let config = SessionConfig {
        warmup_iterations: 10,
        max_iterations: 20,
        ..Default::default()
    };
    let registry = ComponentRegistry::with_defaults();
    let mut session = YoloSession::new(config, registry);

    assert_eq!(session.phase(), SessionPhase::NotStarted);

    session.start();
    assert_eq!(session.phase(), SessionPhase::Warmup);

    // Complete warmup
    for _ in 0..10 {
        let genome = session.next_sample().unwrap();
        session.report_result(genome, make_metrics(1.5, 50));
    }

    assert_eq!(session.phase(), SessionPhase::Exploitation);
}

#[test]
fn test_session_exploitation_phase() {
    let config = SessionConfig {
        warmup_iterations: 5,
        max_iterations: 15,
        ..Default::default()
    };
    let registry = ComponentRegistry::with_defaults();
    let mut session = YoloSession::new(config, registry);

    session.start();

    // Complete warmup and exploitation
    for i in 0..15 {
        let genome = session.next_sample().unwrap();
        let score = 0.5 + (i as f64 / 20.0);
        session.report_result(genome, make_metrics(score * 2.0, 50));
    }

    assert_eq!(session.phase(), SessionPhase::Completed);
}

// =============================================================================
// Sampler determinism tests
// =============================================================================

#[test]
fn test_sampler_determinism() {
    let registry = ComponentRegistry::with_defaults();
    let config = SamplerConfig::default();

    let mut sampler1 = StructuralSampler::with_seed(config.clone(), 42);
    let mut sampler2 = StructuralSampler::with_seed(config, 42);

    for _ in 0..10 {
        let g1 = sampler1.sample_uniform(&registry);
        let g2 = sampler2.sample_uniform(&registry);
        assert_eq!(g1.fingerprint(), g2.fingerprint());
    }
}

#[test]
fn test_structural_mutation_changes_component() {
    let registry = ComponentRegistry::with_defaults();
    let mut sampler = StructuralSampler::with_seed(
        SamplerConfig {
            structural_mutation_rate: 1.0,
            ..Default::default()
        },
        42,
    );

    let original = sampler.sample_uniform(&registry);

    // Try multiple mutations until we see a change
    let mut found_change = false;
    for _ in 0..50 {
        let mutated = sampler.mutate(&original, &registry);

        let orig_sig = original.structural_signature();
        let mut_sig = mutated.structural_signature();

        if orig_sig != mut_sig {
            found_change = true;
            break;
        }
    }

    // With forced structural mutation, we should eventually see a change
    // (unless registry only has one of each component)
    assert!(
        found_change || registry.structural_space_size() <= 4,
        "Should see structural change with structural_mutation_rate=1.0"
    );
}

#[test]
fn test_parameter_mutation_preserves_structure() {
    let registry = ComponentRegistry::with_defaults();
    let mut sampler = StructuralSampler::with_seed(
        SamplerConfig {
            structural_mutation_rate: 0.0,
            param_mutation_rate: 1.0,
            ..Default::default()
        },
        42,
    );

    let original = sampler.sample_uniform(&registry);

    for _ in 0..10 {
        let mutated = sampler.mutate(&original, &registry);

        assert_eq!(original.signal_generator.id, mutated.signal_generator.id);
        assert_eq!(original.position_manager.id, mutated.position_manager.id);
        assert_eq!(original.execution_model.id, mutated.execution_model.id);
    }
}

// =============================================================================
// Component coverage tests
// =============================================================================

#[test]
fn test_all_components_reachable() {
    let registry = ComponentRegistry::with_defaults();
    let mut sampler = StructuralSampler::with_seed(SamplerConfig::default(), 42);

    let mut sampled_sgs = std::collections::HashSet::new();
    let mut sampled_pms = std::collections::HashSet::new();
    let mut sampled_ems = std::collections::HashSet::new();

    for _ in 0..100 {
        let g = sampler.sample_uniform(&registry);
        sampled_sgs.insert(g.signal_generator.id.0.clone());
        sampled_pms.insert(g.position_manager.id.0.clone());
        sampled_ems.insert(g.execution_model.id.0.clone());
    }

    // All registered components should be reachable
    for id in registry.signal_generator_ids() {
        assert!(
            sampled_sgs.contains(&id.0),
            "Signal generator {} not sampled",
            id.0
        );
    }
    for id in registry.position_manager_ids() {
        assert!(
            sampled_pms.contains(&id.0),
            "Position manager {} not sampled",
            id.0
        );
    }
    for id in registry.execution_model_ids() {
        assert!(
            sampled_ems.contains(&id.0),
            "Execution model {} not sampled",
            id.0
        );
    }
}

// =============================================================================
// Leaderboard invariant E tests
// =============================================================================

#[test]
fn test_invariant_e_requires_all_boards() {
    let mut leaderboards = LeaderboardSet::new(100);

    assert!(!leaderboards.invariant_e_satisfied());

    let genome = Genome::new(
        ComponentConfig::new("donchian_breakout"),
        ComponentConfig::new("atr_trailing_stop"),
        ComponentConfig::new("next_open_fill"),
        None,
    );
    let robustness = RobustnessScore {
        score: 0.5,
        is_valid: true,
        ..Default::default()
    };

    // Add to Overall only
    leaderboards.submit(LeaderboardType::Overall, genome.clone(), robustness.clone());
    assert!(!leaderboards.invariant_e_satisfied());

    // Add to SignalQuality
    leaderboards.submit(
        LeaderboardType::SignalQuality,
        genome.clone(),
        robustness.clone(),
    );
    assert!(!leaderboards.invariant_e_satisfied());

    // Add to PositionManagement
    leaderboards.submit(
        LeaderboardType::PositionManagement,
        genome.clone(),
        robustness.clone(),
    );
    assert!(!leaderboards.invariant_e_satisfied());

    // Add to ExecutionSensitivity - now satisfied
    leaderboards.submit(
        LeaderboardType::ExecutionSensitivity,
        genome,
        robustness,
    );
    assert!(leaderboards.invariant_e_satisfied());
}

// =============================================================================
// Robustness scoring tests
// =============================================================================

#[test]
fn test_low_trade_count_invalidates() {
    let config = RobustnessConfig {
        min_trades: 20,
        ..Default::default()
    };
    let scorer = RobustnessScorer::new(config);

    let mut input = RobustnessInput::default();
    input.add_run(&make_metrics(2.0, 5)); // Only 5 trades

    let score = scorer.score(&input);
    assert!(!score.is_valid, "Low trade count should invalidate");
}

#[test]
fn test_high_drawdown_penalizes_score() {
    let config = RobustnessConfig::default();
    let scorer = RobustnessScorer::new(config);

    // Low drawdown result
    let mut input_low_dd = RobustnessInput::default();
    let mut metrics_low = make_metrics(1.5, 50);
    metrics_low.max_drawdown = -0.10;
    input_low_dd.add_run(&metrics_low);
    let score_low_dd = scorer.score(&input_low_dd);

    // High drawdown result
    let mut input_high_dd = RobustnessInput::default();
    let mut metrics_high = make_metrics(1.5, 50);
    metrics_high.max_drawdown = -0.30;
    input_high_dd.add_run(&metrics_high);
    let score_high_dd = scorer.score(&input_high_dd);

    assert!(
        score_high_dd.score < score_low_dd.score,
        "High drawdown should produce lower score: {} vs {}",
        score_high_dd.score,
        score_low_dd.score
    );
}

// =============================================================================
// Attribution tests
// =============================================================================

#[test]
fn test_attribution_marginal_contribution() {
    let mut attr = ComponentAttribution::new();

    // Good signal generator
    for i in 0..10 {
        let genome = Genome::new(
            ComponentConfig::new("sg_good"),
            ComponentConfig::new("pm1"),
            ComponentConfig::new("em1"),
            None,
        );
        attr.add(&genome, 0.8 + (i as f64 - 5.0) * 0.02);
    }

    // Bad signal generator
    for i in 0..10 {
        let genome = Genome::new(
            ComponentConfig::new("sg_bad"),
            ComponentConfig::new("pm1"),
            ComponentConfig::new("em1"),
            None,
        );
        attr.add(&genome, 0.3 + (i as f64 - 5.0) * 0.02);
    }

    let result = attr.compute();

    let sg_good = result
        .signal_generators
        .iter()
        .find(|c| c.id.0 == "sg_good")
        .expect("sg_good not found");
    let sg_bad = result
        .signal_generators
        .iter()
        .find(|c| c.id.0 == "sg_bad")
        .expect("sg_bad not found");

    assert!(
        sg_good.marginal_contribution > 0.0,
        "Good SG should have positive contribution"
    );
    assert!(
        sg_bad.marginal_contribution < 0.0,
        "Bad SG should have negative contribution"
    );
}

#[test]
fn test_top_components() {
    let mut attr = ComponentAttribution::new();

    // Add varied performance
    attr.add(
        &Genome::new(
            ComponentConfig::new("sg1"),
            ComponentConfig::new("pm1"),
            ComponentConfig::new("em1"),
            None,
        ),
        0.9,
    );
    attr.add(
        &Genome::new(
            ComponentConfig::new("sg2"),
            ComponentConfig::new("pm1"),
            ComponentConfig::new("em1"),
            None,
        ),
        0.3,
    );
    attr.add(
        &Genome::new(
            ComponentConfig::new("sg1"),
            ComponentConfig::new("pm2"),
            ComponentConfig::new("em1"),
            None,
        ),
        0.8,
    );

    let top = attr.top_components(2);
    assert_eq!(top.len(), 2);
    assert!(top[0].marginal_contribution >= top[1].marginal_contribution);
}
