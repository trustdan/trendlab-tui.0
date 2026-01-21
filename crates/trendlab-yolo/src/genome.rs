//! Genome representation for strategy compositions.
//!
//! A [`Genome`] encodes a complete strategy composition:
//! - Component types (SignalGenerator, PositionManager, ExecutionModel, SignalFilter)
//! - Parameter values for each component
//!
//! This enables structural Monte Carlo sampling where we vary **structure first**
//! (swap components), then **jitter parameters**.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use trendlab_core::ParamValue;

/// Unique identifier for a component type.
///
/// Each component (signal generator, position manager, etc.) has a unique ID
/// that the registry uses to construct instances.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl ComponentId {
    /// Create a new component ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ComponentId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ComponentId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Component configuration (ID + parameters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// Component identifier (e.g., "donchian_breakout")
    pub id: ComponentId,
    /// Parameter values keyed by name
    pub params: HashMap<String, ParamValue>,
}

impl ComponentConfig {
    /// Create a new component config with default parameters.
    pub fn new(id: impl Into<ComponentId>) -> Self {
        Self {
            id: id.into(),
            params: HashMap::new(),
        }
    }

    /// Create with specific parameters.
    pub fn with_params(id: impl Into<ComponentId>, params: HashMap<String, ParamValue>) -> Self {
        Self {
            id: id.into(),
            params,
        }
    }

    /// Add a parameter value.
    pub fn param(mut self, name: impl Into<String>, value: ParamValue) -> Self {
        self.params.insert(name.into(), value);
        self
    }
}

/// A genome encoding a complete strategy composition.
///
/// The genome represents the "DNA" of a strategy - all the information needed
/// to construct and run a backtest. This includes:
///
/// - Which signal generator to use and its parameters
/// - Which position manager to use and its parameters
/// - Which execution model to use and its parameters
/// - Optional signal filter and its parameters
///
/// # Structural Space
///
/// With 11 signal generators × 10 position managers × 4 execution models × 4 filters,
/// there are 1,760 structural combinations before parameter variation.
///
/// # Example
///
/// ```ignore
/// use trendlab_yolo::{Genome, ComponentConfig};
/// use trendlab_core::ParamValue;
///
/// let genome = Genome::new(
///     ComponentConfig::new("donchian_breakout")
///         .param("lookback", ParamValue::Int(20)),
///     ComponentConfig::new("atr_trailing_stop")
///         .param("multiplier", ParamValue::Float(2.0)),
///     ComponentConfig::new("next_open_fill"),
///     None,
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    /// Signal generator configuration
    pub signal_generator: ComponentConfig,
    /// Position manager configuration
    pub position_manager: ComponentConfig,
    /// Execution model configuration
    pub execution_model: ComponentConfig,
    /// Optional signal filter configuration
    pub signal_filter: Option<ComponentConfig>,
    /// Random seed for reproducibility (if applicable)
    pub seed: Option<u64>,
}

impl Genome {
    /// Create a new genome with the specified components.
    pub fn new(
        signal_generator: ComponentConfig,
        position_manager: ComponentConfig,
        execution_model: ComponentConfig,
        signal_filter: Option<ComponentConfig>,
    ) -> Self {
        Self {
            signal_generator,
            position_manager,
            execution_model,
            signal_filter,
            seed: None,
        }
    }

    /// Set the random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Get a stable fingerprint for this genome.
    ///
    /// Two genomes with identical components and parameters will have the same fingerprint.
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash component IDs
        self.signal_generator.id.0.hash(&mut hasher);
        self.position_manager.id.0.hash(&mut hasher);
        self.execution_model.id.0.hash(&mut hasher);
        if let Some(ref filter) = self.signal_filter {
            filter.id.0.hash(&mut hasher);
        }

        // Hash parameters (sorted for stability)
        let mut sg_params: Vec<_> = self.signal_generator.params.iter().collect();
        sg_params.sort_by_key(|(k, _)| *k);
        for (k, v) in sg_params {
            k.hash(&mut hasher);
            format!("{:?}", v).hash(&mut hasher);
        }

        let mut pm_params: Vec<_> = self.position_manager.params.iter().collect();
        pm_params.sort_by_key(|(k, _)| *k);
        for (k, v) in pm_params {
            k.hash(&mut hasher);
            format!("{:?}", v).hash(&mut hasher);
        }

        let mut em_params: Vec<_> = self.execution_model.params.iter().collect();
        em_params.sort_by_key(|(k, _)| *k);
        for (k, v) in em_params {
            k.hash(&mut hasher);
            format!("{:?}", v).hash(&mut hasher);
        }

        if let Some(ref filter) = self.signal_filter {
            let mut sf_params: Vec<_> = filter.params.iter().collect();
            sf_params.sort_by_key(|(k, _)| *k);
            for (k, v) in sf_params {
                k.hash(&mut hasher);
                format!("{:?}", v).hash(&mut hasher);
            }
        }

        format!("{:016x}", hasher.finish())
    }

    /// Get a human-readable description of this genome.
    pub fn description(&self) -> String {
        let filter_str = self
            .signal_filter
            .as_ref()
            .map(|f| format!(" + {}", f.id))
            .unwrap_or_default();

        format!(
            "{} → {} → {}{}",
            self.signal_generator.id, self.position_manager.id, self.execution_model.id, filter_str
        )
    }

    /// Get the structural signature (component IDs only, no parameters).
    ///
    /// This is used for structural grouping in leaderboards.
    pub fn structural_signature(&self) -> String {
        let filter_id = self
            .signal_filter
            .as_ref()
            .map(|f| f.id.0.as_str())
            .unwrap_or("none");

        format!(
            "{}:{}:{}:{}",
            self.signal_generator.id.0,
            self.position_manager.id.0,
            self.execution_model.id.0,
            filter_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genome_fingerprint_stability() {
        let genome1 = Genome::new(
            ComponentConfig::new("donchian")
                .param("lookback", ParamValue::Int(20)),
            ComponentConfig::new("atr_trailing")
                .param("multiplier", ParamValue::Float(2.0)),
            ComponentConfig::new("next_open"),
            None,
        );

        let genome2 = Genome::new(
            ComponentConfig::new("donchian")
                .param("lookback", ParamValue::Int(20)),
            ComponentConfig::new("atr_trailing")
                .param("multiplier", ParamValue::Float(2.0)),
            ComponentConfig::new("next_open"),
            None,
        );

        assert_eq!(genome1.fingerprint(), genome2.fingerprint());
    }

    #[test]
    fn test_genome_fingerprint_differs_on_param_change() {
        let genome1 = Genome::new(
            ComponentConfig::new("donchian")
                .param("lookback", ParamValue::Int(20)),
            ComponentConfig::new("atr_trailing"),
            ComponentConfig::new("next_open"),
            None,
        );

        let genome2 = Genome::new(
            ComponentConfig::new("donchian")
                .param("lookback", ParamValue::Int(30)), // Different!
            ComponentConfig::new("atr_trailing"),
            ComponentConfig::new("next_open"),
            None,
        );

        assert_ne!(genome1.fingerprint(), genome2.fingerprint());
    }

    #[test]
    fn test_structural_signature() {
        let genome = Genome::new(
            ComponentConfig::new("donchian"),
            ComponentConfig::new("atr_trailing"),
            ComponentConfig::new("next_open"),
            Some(ComponentConfig::new("adx_filter")),
        );

        assert_eq!(
            genome.structural_signature(),
            "donchian:atr_trailing:next_open:adx_filter"
        );
    }

    #[test]
    fn test_structural_signature_no_filter() {
        let genome = Genome::new(
            ComponentConfig::new("donchian"),
            ComponentConfig::new("atr_trailing"),
            ComponentConfig::new("next_open"),
            None,
        );

        assert_eq!(
            genome.structural_signature(),
            "donchian:atr_trailing:next_open:none"
        );
    }
}
