//! Leaderboard system for ranking strategies.
//!
//! # Invariant E: Multiple Leaderboards
//!
//! TrendLab maintains three disentangled leaderboards to understand which
//! *component* is driving performance:
//!
//! 1. **Signal Quality**: Varying signals with fixed PM + fixed execution
//! 2. **Position Management**: Fixed signal with varying PM + fixed execution
//! 3. **Execution Sensitivity**: Fixed signal + fixed PM, varying execution
//!
//! Only after examining all three should overall winners be trusted.

use crate::genome::Genome;
use crate::robustness::RobustnessScore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entry in a leaderboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank (1-indexed)
    pub rank: usize,
    /// Genome fingerprint
    pub fingerprint: String,
    /// Genome configuration
    pub genome: Genome,
    /// Robustness score
    pub robustness: RobustnessScore,
    /// Number of evaluations
    pub num_evaluations: usize,
    /// Timestamp of last update
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl LeaderboardEntry {
    /// Create a new leaderboard entry.
    pub fn new(genome: Genome, robustness: RobustnessScore) -> Self {
        Self {
            rank: 0,
            fingerprint: genome.fingerprint(),
            genome,
            robustness,
            num_evaluations: 1,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Update with new evaluation results.
    pub fn update(&mut self, robustness: RobustnessScore) {
        self.robustness = robustness;
        self.num_evaluations += 1;
        self.last_updated = chrono::Utc::now();
    }
}

/// Type of leaderboard for component isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaderboardType {
    /// Signal Quality: varying signals, fixed PM + execution
    SignalQuality,
    /// Position Management: fixed signal, varying PM, fixed execution
    PositionManagement,
    /// Execution Sensitivity: fixed signal + PM, varying execution
    ExecutionSensitivity,
    /// Overall: all components varied
    Overall,
}

impl std::fmt::Display for LeaderboardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaderboardType::SignalQuality => write!(f, "Signal Quality"),
            LeaderboardType::PositionManagement => write!(f, "Position Management"),
            LeaderboardType::ExecutionSensitivity => write!(f, "Execution Sensitivity"),
            LeaderboardType::Overall => write!(f, "Overall"),
        }
    }
}

/// A single leaderboard.
#[derive(Debug, Clone)]
pub struct Leaderboard {
    /// Type of this leaderboard
    pub leaderboard_type: LeaderboardType,
    /// Entries by fingerprint
    entries: HashMap<String, LeaderboardEntry>,
    /// Sorted fingerprints (by robustness score descending)
    sorted_order: Vec<String>,
    /// Maximum entries to keep
    max_entries: usize,
}

impl Leaderboard {
    /// Create a new leaderboard.
    pub fn new(leaderboard_type: LeaderboardType, max_entries: usize) -> Self {
        Self {
            leaderboard_type,
            entries: HashMap::new(),
            sorted_order: Vec::new(),
            max_entries,
        }
    }

    /// Add or update an entry.
    pub fn submit(&mut self, genome: Genome, robustness: RobustnessScore) {
        let fingerprint = genome.fingerprint();

        if let Some(entry) = self.entries.get_mut(&fingerprint) {
            entry.update(robustness);
        } else {
            let entry = LeaderboardEntry::new(genome, robustness);
            self.entries.insert(fingerprint.clone(), entry);
        }

        self.resort();
        self.trim();
    }

    /// Resort entries by robustness score.
    fn resort(&mut self) {
        let mut scored: Vec<_> = self
            .entries
            .iter()
            .map(|(fp, e)| (fp.clone(), e.robustness.score))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        self.sorted_order = scored.into_iter().map(|(fp, _)| fp).collect();

        // Update ranks
        for (rank, fp) in self.sorted_order.iter().enumerate() {
            if let Some(entry) = self.entries.get_mut(fp) {
                entry.rank = rank + 1;
            }
        }
    }

    /// Trim to max entries.
    fn trim(&mut self) {
        while self.sorted_order.len() > self.max_entries {
            if let Some(fp) = self.sorted_order.pop() {
                self.entries.remove(&fp);
            }
        }
    }

    /// Get top N entries.
    pub fn top(&self, n: usize) -> Vec<&LeaderboardEntry> {
        self.sorted_order
            .iter()
            .take(n)
            .filter_map(|fp| self.entries.get(fp))
            .collect()
    }

    /// Get entry by fingerprint.
    pub fn get(&self, fingerprint: &str) -> Option<&LeaderboardEntry> {
        self.entries.get(fingerprint)
    }

    /// Get rank of an entry.
    pub fn rank_of(&self, fingerprint: &str) -> Option<usize> {
        self.entries.get(fingerprint).map(|e| e.rank)
    }

    /// Get total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries in ranked order.
    pub fn iter(&self) -> impl Iterator<Item = &LeaderboardEntry> {
        self.sorted_order
            .iter()
            .filter_map(move |fp| self.entries.get(fp))
    }
}

/// Set of all leaderboards (Invariant E compliance).
///
/// This structure ensures that we have multiple disentangled leaderboards
/// before trusting overall winners.
#[derive(Debug, Clone)]
pub struct LeaderboardSet {
    /// Signal quality leaderboard
    pub signal_quality: Leaderboard,
    /// Position management leaderboard
    pub position_management: Leaderboard,
    /// Execution sensitivity leaderboard
    pub execution_sensitivity: Leaderboard,
    /// Overall leaderboard
    pub overall: Leaderboard,
}

impl LeaderboardSet {
    /// Create a new leaderboard set.
    pub fn new(max_entries_per_board: usize) -> Self {
        Self {
            signal_quality: Leaderboard::new(LeaderboardType::SignalQuality, max_entries_per_board),
            position_management: Leaderboard::new(
                LeaderboardType::PositionManagement,
                max_entries_per_board,
            ),
            execution_sensitivity: Leaderboard::new(
                LeaderboardType::ExecutionSensitivity,
                max_entries_per_board,
            ),
            overall: Leaderboard::new(LeaderboardType::Overall, max_entries_per_board),
        }
    }

    /// Submit to the appropriate leaderboard.
    pub fn submit(
        &mut self,
        leaderboard_type: LeaderboardType,
        genome: Genome,
        robustness: RobustnessScore,
    ) {
        match leaderboard_type {
            LeaderboardType::SignalQuality => {
                self.signal_quality.submit(genome, robustness);
            }
            LeaderboardType::PositionManagement => {
                self.position_management.submit(genome, robustness);
            }
            LeaderboardType::ExecutionSensitivity => {
                self.execution_sensitivity.submit(genome, robustness);
            }
            LeaderboardType::Overall => {
                self.overall.submit(genome, robustness);
            }
        }
    }

    /// Get a leaderboard by type.
    pub fn get(&self, leaderboard_type: LeaderboardType) -> &Leaderboard {
        match leaderboard_type {
            LeaderboardType::SignalQuality => &self.signal_quality,
            LeaderboardType::PositionManagement => &self.position_management,
            LeaderboardType::ExecutionSensitivity => &self.execution_sensitivity,
            LeaderboardType::Overall => &self.overall,
        }
    }

    /// Get mutable reference to a leaderboard by type.
    pub fn get_mut(&mut self, leaderboard_type: LeaderboardType) -> &mut Leaderboard {
        match leaderboard_type {
            LeaderboardType::SignalQuality => &mut self.signal_quality,
            LeaderboardType::PositionManagement => &mut self.position_management,
            LeaderboardType::ExecutionSensitivity => &mut self.execution_sensitivity,
            LeaderboardType::Overall => &mut self.overall,
        }
    }

    /// Check Invariant E: All three component leaderboards must have entries
    /// before the overall leaderboard can be trusted.
    pub fn invariant_e_satisfied(&self) -> bool {
        !self.signal_quality.is_empty()
            && !self.position_management.is_empty()
            && !self.execution_sensitivity.is_empty()
    }

    /// Get summary statistics.
    pub fn summary(&self) -> LeaderboardSetSummary {
        LeaderboardSetSummary {
            signal_quality_count: self.signal_quality.len(),
            position_management_count: self.position_management.len(),
            execution_sensitivity_count: self.execution_sensitivity.len(),
            overall_count: self.overall.len(),
            invariant_e_satisfied: self.invariant_e_satisfied(),
        }
    }
}

impl Default for LeaderboardSet {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Summary statistics for a leaderboard set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardSetSummary {
    /// Number of entries in signal quality leaderboard
    pub signal_quality_count: usize,
    /// Number of entries in position management leaderboard
    pub position_management_count: usize,
    /// Number of entries in execution sensitivity leaderboard
    pub execution_sensitivity_count: usize,
    /// Number of entries in overall leaderboard
    pub overall_count: usize,
    /// Whether Invariant E is satisfied
    pub invariant_e_satisfied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::ComponentConfig;

    fn make_genome(sg: &str, pm: &str, em: &str) -> Genome {
        Genome::new(
            ComponentConfig::new(sg),
            ComponentConfig::new(pm),
            ComponentConfig::new(em),
            None,
        )
    }

    fn make_score(score: f64) -> RobustnessScore {
        RobustnessScore {
            score,
            is_valid: true,
            num_runs: 1,
            ..Default::default()
        }
    }

    #[test]
    fn test_leaderboard_submit_and_rank() {
        let mut lb = Leaderboard::new(LeaderboardType::Overall, 10);

        lb.submit(make_genome("sg1", "pm1", "em1"), make_score(0.5));
        lb.submit(make_genome("sg2", "pm1", "em1"), make_score(0.8));
        lb.submit(make_genome("sg3", "pm1", "em1"), make_score(0.3));

        assert_eq!(lb.len(), 3);

        let top = lb.top(3);
        assert_eq!(top.len(), 3);
        assert!(top[0].robustness.score > top[1].robustness.score);
        assert!(top[1].robustness.score > top[2].robustness.score);
    }

    #[test]
    fn test_leaderboard_update_existing() {
        let mut lb = Leaderboard::new(LeaderboardType::Overall, 10);

        let genome = make_genome("sg1", "pm1", "em1");
        lb.submit(genome.clone(), make_score(0.5));
        lb.submit(genome.clone(), make_score(0.7)); // Update same genome

        assert_eq!(lb.len(), 1);

        let entry = lb.get(&genome.fingerprint()).unwrap();
        assert!((entry.robustness.score - 0.7).abs() < 1e-10);
        assert_eq!(entry.num_evaluations, 2);
    }

    #[test]
    fn test_leaderboard_trim() {
        let mut lb = Leaderboard::new(LeaderboardType::Overall, 3);

        lb.submit(make_genome("sg1", "pm1", "em1"), make_score(0.5));
        lb.submit(make_genome("sg2", "pm1", "em1"), make_score(0.8));
        lb.submit(make_genome("sg3", "pm1", "em1"), make_score(0.3));
        lb.submit(make_genome("sg4", "pm1", "em1"), make_score(0.9)); // Should push out lowest

        assert_eq!(lb.len(), 3);

        // sg3 (0.3) should be gone
        let genome3 = make_genome("sg3", "pm1", "em1");
        assert!(lb.get(&genome3.fingerprint()).is_none());
    }

    #[test]
    fn test_leaderboard_set_invariant_e() {
        let mut set = LeaderboardSet::new(10);

        // Initially not satisfied
        assert!(!set.invariant_e_satisfied());

        // Add to signal quality
        set.submit(
            LeaderboardType::SignalQuality,
            make_genome("sg1", "pm1", "em1"),
            make_score(0.5),
        );
        assert!(!set.invariant_e_satisfied());

        // Add to position management
        set.submit(
            LeaderboardType::PositionManagement,
            make_genome("sg1", "pm1", "em1"),
            make_score(0.5),
        );
        assert!(!set.invariant_e_satisfied());

        // Add to execution sensitivity
        set.submit(
            LeaderboardType::ExecutionSensitivity,
            make_genome("sg1", "pm1", "em1"),
            make_score(0.5),
        );
        assert!(set.invariant_e_satisfied()); // Now satisfied!
    }

    #[test]
    fn test_leaderboard_type_display() {
        assert_eq!(LeaderboardType::SignalQuality.to_string(), "Signal Quality");
        assert_eq!(
            LeaderboardType::PositionManagement.to_string(),
            "Position Management"
        );
        assert_eq!(
            LeaderboardType::ExecutionSensitivity.to_string(),
            "Execution Sensitivity"
        );
        assert_eq!(LeaderboardType::Overall.to_string(), "Overall");
    }
}
