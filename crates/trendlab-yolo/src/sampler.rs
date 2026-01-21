//! Structural Monte Carlo sampler.
//!
//! The sampler explores the combinatorial strategy space by varying
//! **structure first** (which components to combine), then parameters.
//!
//! # Sampling Strategy
//!
//! 1. **Warmup Phase**: Uniform sampling across all structural combinations
//! 2. **Exploitation Phase**: Weight sampling towards high-robustness structures
//!
//! # Mutation Types
//!
//! - **Structural mutation**: Swap one component type (e.g., different signal generator)
//! - **Parameter mutation**: Jitter parameters within their bounds

use crate::genome::{ComponentConfig, ComponentId, Genome};
use crate::registry::ComponentRegistry;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use trendlab_core::param::{ParamType, ParamValue};

/// Configuration for the structural sampler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplerConfig {
    /// Probability of structural mutation vs parameter mutation (default: 0.3)
    pub structural_mutation_rate: f64,
    /// Probability of mutating each parameter (default: 0.2)
    pub param_mutation_rate: f64,
    /// Standard deviation for parameter jitter (relative) (default: 0.1)
    pub param_jitter_std: f64,
    /// Exploitation bias (higher = more exploitation) (default: 2.0)
    pub exploitation_temperature: f64,
}

impl Default for SamplerConfig {
    fn default() -> Self {
        Self {
            structural_mutation_rate: 0.3,
            param_mutation_rate: 0.2,
            param_jitter_std: 0.1,
            exploitation_temperature: 2.0,
        }
    }
}

/// Structural Monte Carlo sampler.
///
/// Explores the strategy space by:
/// 1. Randomly selecting component combinations
/// 2. Sampling parameters within their specified ranges
/// 3. Mutating existing genomes to explore nearby variants
pub struct StructuralSampler<R: Rng = rand::rngs::StdRng> {
    config: SamplerConfig,
    rng: R,
}

impl StructuralSampler<rand::rngs::StdRng> {
    /// Create a new sampler with random seed.
    pub fn new(config: SamplerConfig) -> Self {
        Self {
            config,
            rng: rand::rngs::StdRng::from_os_rng(),
        }
    }

    /// Create a sampler with a specific seed for reproducibility.
    pub fn with_seed(config: SamplerConfig, seed: u64) -> Self {
        Self {
            config,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }
}

impl<R: Rng> StructuralSampler<R> {
    /// Create a sampler with a custom RNG.
    pub fn with_rng(config: SamplerConfig, rng: R) -> Self {
        Self { config, rng }
    }

    /// Sample a random genome uniformly from the structural space.
    ///
    /// Used during warmup phase.
    pub fn sample_uniform(&mut self, registry: &ComponentRegistry) -> Genome {
        let sg_ids = registry.signal_generator_ids();
        let pm_ids = registry.position_manager_ids();
        let em_ids = registry.execution_model_ids();
        let sf_ids = registry.signal_filter_ids();

        let sg = sg_ids.choose(&mut self.rng).unwrap();
        let pm = pm_ids.choose(&mut self.rng).unwrap();
        let em = em_ids.choose(&mut self.rng).unwrap();
        let sf = sf_ids.choose(&mut self.rng);

        // Sample parameters for each component
        let sg_config = self.sample_component_params(registry, sg);
        let pm_config = self.sample_component_params(registry, pm);
        let em_config = self.sample_component_params(registry, em);
        let sf_config = sf.map(|id| self.sample_component_params(registry, id));

        Genome::new(sg_config, pm_config, em_config, sf_config)
            .with_seed(self.rng.random())
    }

    /// Sample component configuration with random parameters.
    fn sample_component_params(
        &mut self,
        registry: &ComponentRegistry,
        id: &ComponentId,
    ) -> ComponentConfig {
        let mut config = ComponentConfig::new(id.clone());

        if let Some(params) = registry.get_params(id) {
            for param_def in params {
                let value = self.sample_param_value(&param_def.param_type);
                config.params.insert(param_def.name.clone(), value);
            }
        }

        config
    }

    /// Sample a parameter value within its bounds.
    fn sample_param_value(&mut self, param_type: &ParamType) -> ParamValue {
        match param_type {
            ParamType::Int { min, max, step } => {
                let steps = (max - min) / step;
                let n = self.rng.random_range(0..=steps);
                ParamValue::Int(min + n * step)
            }
            ParamType::Float { min, max, step } => {
                let steps = ((max - min) / step) as i64;
                let n = self.rng.random_range(0..=steps) as f64;
                ParamValue::Float(min + n * step)
            }
            ParamType::Bool => ParamValue::Bool(self.rng.random()),
            ParamType::Choice(choices) => {
                let choice = choices.choose(&mut self.rng).unwrap();
                ParamValue::Choice(choice.clone())
            }
        }
    }

    /// Mutate an existing genome.
    ///
    /// With probability `structural_mutation_rate`, swap one component.
    /// Otherwise, jitter parameters.
    pub fn mutate(&mut self, genome: &Genome, registry: &ComponentRegistry) -> Genome {
        if self.rng.random::<f64>() < self.config.structural_mutation_rate {
            self.structural_mutate(genome, registry)
        } else {
            self.param_mutate(genome, registry)
        }
    }

    /// Perform a structural mutation (swap one component).
    fn structural_mutate(&mut self, genome: &Genome, registry: &ComponentRegistry) -> Genome {
        // Choose which component type to mutate
        let component_type = self.rng.random_range(0..4);

        let mut new_genome = genome.clone();

        match component_type {
            0 => {
                // Mutate signal generator
                let ids = registry.signal_generator_ids();
                let new_id = ids.choose(&mut self.rng).unwrap();
                new_genome.signal_generator = self.sample_component_params(registry, new_id);
            }
            1 => {
                // Mutate position manager
                let ids = registry.position_manager_ids();
                let new_id = ids.choose(&mut self.rng).unwrap();
                new_genome.position_manager = self.sample_component_params(registry, new_id);
            }
            2 => {
                // Mutate execution model
                let ids = registry.execution_model_ids();
                let new_id = ids.choose(&mut self.rng).unwrap();
                new_genome.execution_model = self.sample_component_params(registry, new_id);
            }
            _ => {
                // Mutate signal filter
                let ids = registry.signal_filter_ids();
                if self.rng.random::<bool>() && !ids.is_empty() {
                    let new_id = ids.choose(&mut self.rng).unwrap();
                    new_genome.signal_filter =
                        Some(self.sample_component_params(registry, new_id));
                } else {
                    new_genome.signal_filter = None;
                }
            }
        }

        new_genome.seed = Some(self.rng.random());
        new_genome
    }

    /// Perform a parameter mutation (jitter existing parameters).
    fn param_mutate(&mut self, genome: &Genome, registry: &ComponentRegistry) -> Genome {
        let mut new_genome = genome.clone();

        // Mutate signal generator params
        self.mutate_component_params(&mut new_genome.signal_generator, registry);

        // Mutate position manager params
        self.mutate_component_params(&mut new_genome.position_manager, registry);

        // Mutate execution model params
        self.mutate_component_params(&mut new_genome.execution_model, registry);

        // Mutate signal filter params (if present)
        if let Some(ref mut filter) = new_genome.signal_filter {
            self.mutate_component_params(filter, registry);
        }

        new_genome.seed = Some(self.rng.random());
        new_genome
    }

    /// Mutate parameters in a component config.
    fn mutate_component_params(
        &mut self,
        config: &mut ComponentConfig,
        registry: &ComponentRegistry,
    ) {
        if let Some(param_defs) = registry.get_params(&config.id) {
            for param_def in param_defs {
                if self.rng.random::<f64>() < self.config.param_mutation_rate {
                    if let Some(value) = config.params.get(&param_def.name) {
                        let mutated = self.jitter_param(value, &param_def.param_type);
                        config.params.insert(param_def.name.clone(), mutated);
                    }
                }
            }
        }
    }

    /// Jitter a parameter value.
    fn jitter_param(&mut self, value: &ParamValue, param_type: &ParamType) -> ParamValue {
        match (value, param_type) {
            (ParamValue::Int(v), ParamType::Int { min, max, step }) => {
                let jitter = (self.rng.random::<f64>() - 0.5)
                    * 2.0
                    * self.config.param_jitter_std
                    * (*max - *min) as f64;
                let jittered = *v + (jitter / *step as f64).round() as i64 * step;
                ParamValue::Int(jittered.clamp(*min, *max))
            }
            (ParamValue::Float(v), ParamType::Float { min, max, step }) => {
                let jitter =
                    (self.rng.random::<f64>() - 0.5) * 2.0 * self.config.param_jitter_std * (max - min);
                let jittered = v + jitter;
                // Snap to step
                let snapped = (jittered / step).round() * step;
                ParamValue::Float(snapped.clamp(*min, *max))
            }
            (ParamValue::Bool(_), ParamType::Bool) => {
                // Flip with probability
                if self.rng.random::<f64>() < 0.5 {
                    ParamValue::Bool(self.rng.random())
                } else {
                    value.clone()
                }
            }
            (_, ParamType::Choice(choices)) => {
                // Pick a random different choice
                let choice = choices.choose(&mut self.rng).unwrap();
                ParamValue::Choice(choice.clone())
            }
            _ => value.clone(),
        }
    }

    /// Sample with exploitation bias based on scores.
    ///
    /// Higher scores get higher sampling probability.
    pub fn sample_exploitation(
        &mut self,
        candidates: &[(Genome, f64)],
        registry: &ComponentRegistry,
    ) -> Genome {
        if candidates.is_empty() {
            return self.sample_uniform(registry);
        }

        // Compute softmax-like weights
        let max_score = candidates
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::NEG_INFINITY, f64::max);

        let weights: Vec<f64> = candidates
            .iter()
            .map(|(_, s)| ((s - max_score) * self.config.exploitation_temperature).exp())
            .collect();

        let total_weight: f64 = weights.iter().sum();

        // Sample according to weights
        let mut sample = self.rng.random::<f64>() * total_weight;
        for (i, weight) in weights.iter().enumerate() {
            sample -= weight;
            if sample <= 0.0 {
                // Mutate the selected genome
                return self.mutate(&candidates[i].0, registry);
            }
        }

        // Fallback: mutate the last one
        self.mutate(&candidates[candidates.len() - 1].0, registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_uniform() {
        let registry = ComponentRegistry::with_defaults();
        let mut sampler = StructuralSampler::with_seed(SamplerConfig::default(), 42);

        let genome = sampler.sample_uniform(&registry);

        // Should have valid component IDs
        assert!(!genome.signal_generator.id.0.is_empty());
        assert!(!genome.position_manager.id.0.is_empty());
        assert!(!genome.execution_model.id.0.is_empty());
    }

    #[test]
    fn test_sample_determinism() {
        let registry = ComponentRegistry::with_defaults();
        let config = SamplerConfig::default();

        let mut sampler1 = StructuralSampler::with_seed(config.clone(), 42);
        let mut sampler2 = StructuralSampler::with_seed(config, 42);

        let genome1 = sampler1.sample_uniform(&registry);
        let genome2 = sampler2.sample_uniform(&registry);

        assert_eq!(genome1.fingerprint(), genome2.fingerprint());
    }

    #[test]
    fn test_structural_mutation_changes_component() {
        let registry = ComponentRegistry::with_defaults();
        let mut sampler = StructuralSampler::with_seed(SamplerConfig::default(), 42);

        let original = sampler.sample_uniform(&registry);

        // Force multiple structural mutations until one actually changes
        let mut different = false;
        for _ in 0..100 {
            let mutated = sampler.structural_mutate(&original, &registry);
            if mutated.structural_signature() != original.structural_signature() {
                different = true;
                break;
            }
        }

        // With enough tries, we should get a different structure
        // (unless registry only has 1 of each component)
        assert!(different || registry.structural_space_size() == 1);
    }

    #[test]
    fn test_param_mutation_preserves_structure() {
        let registry = ComponentRegistry::with_defaults();
        let mut sampler = StructuralSampler::with_seed(SamplerConfig::default(), 42);

        let original = sampler.sample_uniform(&registry);
        let mutated = sampler.param_mutate(&original, &registry);

        // Structure should be the same
        assert_eq!(
            mutated.signal_generator.id,
            original.signal_generator.id
        );
        assert_eq!(
            mutated.position_manager.id,
            original.position_manager.id
        );
        assert_eq!(
            mutated.execution_model.id,
            original.execution_model.id
        );
    }

    #[test]
    fn test_exploitation_sampling() {
        let registry = ComponentRegistry::with_defaults();
        let mut sampler = StructuralSampler::with_seed(SamplerConfig::default(), 42);

        // Create candidates with varying scores
        let candidates: Vec<(Genome, f64)> = (0..5)
            .map(|i| {
                let genome = sampler.sample_uniform(&registry);
                (genome, i as f64 * 0.2)
            })
            .collect();

        // Sample many times
        let mut samples = Vec::new();
        for _ in 0..100 {
            let sample = sampler.sample_exploitation(&candidates, &registry);
            samples.push(sample);
        }

        // Should have produced valid genomes
        assert!(!samples.is_empty());
        for sample in &samples {
            assert!(!sample.signal_generator.id.0.is_empty());
        }
    }

    #[test]
    fn test_sample_param_values() {
        let config = SamplerConfig::default();
        let mut sampler = StructuralSampler::with_seed(config, 42);

        // Test int sampling
        for _ in 0..100 {
            let value = sampler.sample_param_value(&ParamType::Int {
                min: 10,
                max: 50,
                step: 5,
            });
            if let ParamValue::Int(v) = value {
                assert!(v >= 10 && v <= 50);
                assert!((v - 10) % 5 == 0);
            } else {
                panic!("Expected Int value");
            }
        }

        // Test float sampling
        for _ in 0..100 {
            let value = sampler.sample_param_value(&ParamType::Float {
                min: 1.0,
                max: 3.0,
                step: 0.5,
            });
            if let ParamValue::Float(v) = value {
                assert!(v >= 1.0 && v <= 3.0);
            } else {
                panic!("Expected Float value");
            }
        }

        // Test bool sampling
        let mut seen_true = false;
        let mut seen_false = false;
        for _ in 0..100 {
            let value = sampler.sample_param_value(&ParamType::Bool);
            if let ParamValue::Bool(v) = value {
                if v {
                    seen_true = true;
                } else {
                    seen_false = true;
                }
            }
        }
        assert!(seen_true && seen_false);
    }
}
