//! YOLO session state machine.
//!
//! A YOLO session manages the exploration lifecycle:
//!
//! 1. **Warmup Phase**: Uniform sampling to build initial population
//! 2. **Exploitation Phase**: Biased sampling towards high-robustness strategies
//! 3. **Completion**: Session ends when time/iteration budget exhausted
//!
//! # Usage
//!
//! ```ignore
//! let config = SessionConfig::default();
//! let mut session = YoloSession::new(config, registry);
//!
//! session.start();
//!
//! while session.is_running() {
//!     let genome = session.next_sample();
//!     let result = run_backtest(&genome);
//!     session.report_result(genome, result);
//! }
//!
//! let leaderboards = session.leaderboards();
//! ```

use crate::genome::Genome;
use crate::leaderboard::{LeaderboardSet, LeaderboardType};
use crate::registry::ComponentRegistry;
use crate::robustness::{RobustnessConfig, RobustnessInput, RobustnessScore, RobustnessScorer};
use crate::sampler::{SamplerConfig, StructuralSampler};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use trendlab_core::Metrics;

/// Configuration for a YOLO session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Number of warmup iterations (uniform sampling)
    pub warmup_iterations: usize,
    /// Maximum total iterations (0 = unlimited)
    pub max_iterations: usize,
    /// Maximum session duration (0 = unlimited)
    #[serde(with = "duration_serde")]
    pub max_duration: Duration,
    /// Sampler configuration
    pub sampler_config: SamplerConfig,
    /// Robustness scorer configuration
    pub robustness_config: RobustnessConfig,
    /// Maximum entries per leaderboard
    pub max_leaderboard_entries: usize,
    /// Random seed for reproducibility (None = random)
    pub seed: Option<u64>,
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 50,
            max_iterations: 1000,
            max_duration: Duration::from_secs(300), // 5 minutes
            sampler_config: SamplerConfig::default(),
            robustness_config: RobustnessConfig::default(),
            max_leaderboard_entries: 100,
            seed: None,
        }
    }
}

/// Current phase of the YOLO session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionPhase {
    /// Not yet started
    NotStarted,
    /// Warmup phase (uniform sampling)
    Warmup,
    /// Exploitation phase (biased sampling)
    Exploitation,
    /// Session complete
    Completed,
}

impl std::fmt::Display for SessionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionPhase::NotStarted => write!(f, "Not Started"),
            SessionPhase::Warmup => write!(f, "Warmup"),
            SessionPhase::Exploitation => write!(f, "Exploitation"),
            SessionPhase::Completed => write!(f, "Completed"),
        }
    }
}

/// Statistics for the current session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total iterations completed
    pub iterations: usize,
    /// Warmup iterations completed
    pub warmup_complete: usize,
    /// Exploitation iterations completed
    pub exploitation_complete: usize,
    /// Number of valid results
    pub valid_results: usize,
    /// Number of invalid results (insufficient trades, etc.)
    pub invalid_results: usize,
    /// Best robustness score seen
    pub best_score: f64,
    /// Average robustness score
    pub avg_score: f64,
    /// Elapsed time
    #[serde(with = "duration_serde")]
    pub elapsed: Duration,
}

/// YOLO session state machine.
///
/// Manages the exploration lifecycle from warmup through exploitation.
pub struct YoloSession {
    config: SessionConfig,
    phase: SessionPhase,
    stats: SessionStats,
    start_time: Option<Instant>,
    leaderboards: LeaderboardSet,
    sampler: StructuralSampler,
    scorer: RobustnessScorer,
    registry: ComponentRegistry,
    /// Cached top genomes for exploitation sampling
    top_genomes: Vec<(Genome, f64)>,
}

impl YoloSession {
    /// Create a new YOLO session.
    pub fn new(config: SessionConfig, registry: ComponentRegistry) -> Self {
        let seed = config.seed.unwrap_or_else(rand::random);
        let sampler = StructuralSampler::with_seed(config.sampler_config.clone(), seed);
        let scorer = RobustnessScorer::new(config.robustness_config.clone());
        let leaderboards = LeaderboardSet::new(config.max_leaderboard_entries);

        Self {
            config,
            phase: SessionPhase::NotStarted,
            stats: SessionStats::default(),
            start_time: None,
            leaderboards,
            sampler,
            scorer,
            registry,
            top_genomes: Vec::new(),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SessionConfig::default(), ComponentRegistry::with_defaults())
    }

    /// Start the session.
    pub fn start(&mut self) {
        self.phase = SessionPhase::Warmup;
        self.start_time = Some(Instant::now());
        self.stats = SessionStats::default();
    }

    /// Check if the session is running.
    pub fn is_running(&self) -> bool {
        matches!(self.phase, SessionPhase::Warmup | SessionPhase::Exploitation)
    }

    /// Get the current phase.
    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    /// Get session statistics.
    pub fn stats(&self) -> &SessionStats {
        &self.stats
    }

    /// Get current elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// Get progress as fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.config.max_iterations > 0 {
            self.stats.iterations as f64 / self.config.max_iterations as f64
        } else if self.config.max_duration.as_secs() > 0 {
            self.elapsed().as_secs_f64() / self.config.max_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Get the next genome to sample.
    ///
    /// Returns None if the session is complete.
    pub fn next_sample(&mut self) -> Option<Genome> {
        self.check_phase_transition();

        if !self.is_running() {
            return None;
        }

        let genome = match self.phase {
            SessionPhase::Warmup => self.sampler.sample_uniform(&self.registry),
            SessionPhase::Exploitation => {
                if self.top_genomes.is_empty() {
                    self.sampler.sample_uniform(&self.registry)
                } else {
                    self.sampler.sample_exploitation(&self.top_genomes, &self.registry)
                }
            }
            _ => return None,
        };

        Some(genome)
    }

    /// Report a backtest result.
    ///
    /// The session will score the result and update leaderboards.
    pub fn report_result(&mut self, genome: Genome, metrics: Metrics) {
        let input = RobustnessInput::from_metrics(&metrics);
        let score = self.scorer.score(&input);

        self.update_stats(&score);

        if score.is_valid {
            // Submit to appropriate leaderboard
            // For now, submit to overall. Component isolation logic would determine
            // which leaderboard based on what was varied.
            self.leaderboards.submit(
                LeaderboardType::Overall,
                genome.clone(),
                score.clone(),
            );

            // Update top genomes cache for exploitation
            self.top_genomes.push((genome, score.score));
            self.top_genomes
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            self.top_genomes.truncate(20);
        }

        self.check_phase_transition();
    }

    /// Report multiple results for the same genome (cross-validation).
    pub fn report_results(&mut self, genome: Genome, metrics_list: &[Metrics]) {
        let mut input = RobustnessInput::default();
        for metrics in metrics_list {
            input.add_run(metrics);
        }

        let score = self.scorer.score(&input);
        self.update_stats(&score);

        if score.is_valid {
            self.leaderboards.submit(
                LeaderboardType::Overall,
                genome.clone(),
                score.clone(),
            );

            self.top_genomes.push((genome, score.score));
            self.top_genomes
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            self.top_genomes.truncate(20);
        }

        self.check_phase_transition();
    }

    /// Update session statistics.
    fn update_stats(&mut self, score: &RobustnessScore) {
        self.stats.iterations += 1;
        self.stats.elapsed = self.elapsed();

        if matches!(self.phase, SessionPhase::Warmup) {
            self.stats.warmup_complete += 1;
        } else {
            self.stats.exploitation_complete += 1;
        }

        if score.is_valid {
            self.stats.valid_results += 1;

            // Update best score
            if score.score > self.stats.best_score {
                self.stats.best_score = score.score;
            }

            // Update running average
            let n = self.stats.valid_results as f64;
            self.stats.avg_score = (self.stats.avg_score * (n - 1.0) + score.score) / n;
        } else {
            self.stats.invalid_results += 1;
        }
    }

    /// Check and perform phase transitions.
    fn check_phase_transition(&mut self) {
        match self.phase {
            SessionPhase::Warmup => {
                if self.stats.warmup_complete >= self.config.warmup_iterations {
                    self.phase = SessionPhase::Exploitation;
                }
            }
            SessionPhase::Exploitation => {
                let should_complete = (self.config.max_iterations > 0
                    && self.stats.iterations >= self.config.max_iterations)
                    || (self.config.max_duration.as_secs() > 0
                        && self.elapsed() >= self.config.max_duration);

                if should_complete {
                    self.phase = SessionPhase::Completed;
                }
            }
            _ => {}
        }
    }

    /// Get leaderboards.
    pub fn leaderboards(&self) -> &LeaderboardSet {
        &self.leaderboards
    }

    /// Get registry.
    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    /// Get configuration.
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Stop the session.
    pub fn stop(&mut self) {
        self.phase = SessionPhase::Completed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metrics(sharpe: f64, trades: usize) -> Metrics {
        Metrics {
            sharpe,
            total_trades: trades,
            win_rate: 0.55,
            total_return: 0.20,
            max_drawdown: -0.10,
            ..Default::default()
        }
    }

    #[test]
    fn test_session_lifecycle() {
        let config = SessionConfig {
            warmup_iterations: 5,
            max_iterations: 10,
            ..Default::default()
        };

        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());

        assert_eq!(session.phase(), SessionPhase::NotStarted);

        session.start();
        assert_eq!(session.phase(), SessionPhase::Warmup);
        assert!(session.is_running());

        // Complete warmup
        for _ in 0..5 {
            let genome = session.next_sample().unwrap();
            session.report_result(genome, make_metrics(1.5, 50));
        }

        assert_eq!(session.phase(), SessionPhase::Exploitation);

        // Complete exploitation
        for _ in 0..5 {
            let genome = session.next_sample().unwrap();
            session.report_result(genome, make_metrics(1.5, 50));
        }

        assert_eq!(session.phase(), SessionPhase::Completed);
        assert!(!session.is_running());
    }

    #[test]
    fn test_session_stats() {
        let config = SessionConfig {
            warmup_iterations: 3,
            max_iterations: 10,
            ..Default::default()
        };

        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());
        session.start();

        for _ in 0..3 {
            let genome = session.next_sample().unwrap();
            session.report_result(genome, make_metrics(1.5, 50));
        }

        let stats = session.stats();
        assert_eq!(stats.iterations, 3);
        assert_eq!(stats.warmup_complete, 3);
        assert!(stats.valid_results > 0);
    }

    #[test]
    fn test_session_stop() {
        let mut session = YoloSession::with_defaults();
        session.start();

        assert!(session.is_running());

        session.stop();

        assert!(!session.is_running());
        assert_eq!(session.phase(), SessionPhase::Completed);
    }

    #[test]
    fn test_session_leaderboard_updates() {
        let config = SessionConfig {
            warmup_iterations: 3,
            max_iterations: 10,
            ..Default::default()
        };

        let mut session = YoloSession::new(config, ComponentRegistry::with_defaults());
        session.start();

        for _ in 0..5 {
            let genome = session.next_sample().unwrap();
            session.report_result(genome, make_metrics(1.5, 50));
        }

        let leaderboards = session.leaderboards();
        assert!(!leaderboards.overall.is_empty());
    }

    #[test]
    fn test_phase_display() {
        assert_eq!(SessionPhase::NotStarted.to_string(), "Not Started");
        assert_eq!(SessionPhase::Warmup.to_string(), "Warmup");
        assert_eq!(SessionPhase::Exploitation.to_string(), "Exploitation");
        assert_eq!(SessionPhase::Completed.to_string(), "Completed");
    }
}
