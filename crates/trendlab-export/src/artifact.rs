//! Strategy artifact types.
//!
//! These types match the JSON schema at `schemas/strategy-artifact.schema.json`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{ExportError, ExportResult};
use crate::SCHEMA_VERSION;

/// Complete strategy artifact for export.
///
/// Contains all information needed to:
/// - Generate Pine Script v6
/// - Reproduce the backtest
/// - Validate parity between Rust and Pine implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyArtifact {
    /// Schema version (semver).
    pub schema_version: String,
    /// Unique strategy identifier (typically the genome fingerprint).
    pub strategy_id: String,
    /// Export timestamp.
    pub exported_at: DateTime<Utc>,
    /// Signal generator configuration.
    pub signal_generator: ComponentArtifact,
    /// Position manager configuration.
    pub position_manager: ComponentArtifact,
    /// Execution model configuration.
    pub execution_model: ComponentArtifact,
    /// Optional signal filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_filter: Option<ComponentArtifact>,
    /// Backtest results (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtest_results: Option<BacktestResults>,
    /// Parity test vectors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parity_vectors: Option<ParityVectors>,
    /// Run fingerprint for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_fingerprint: Option<String>,
}

impl StrategyArtifact {
    /// Create a new strategy artifact.
    pub fn new(
        strategy_id: String,
        signal_generator: ComponentArtifact,
        position_manager: ComponentArtifact,
        execution_model: ComponentArtifact,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            strategy_id,
            exported_at: Utc::now(),
            signal_generator,
            position_manager,
            execution_model,
            signal_filter: None,
            backtest_results: None,
            parity_vectors: None,
            run_fingerprint: None,
        }
    }

    /// Add a signal filter.
    pub fn with_filter(mut self, filter: ComponentArtifact) -> Self {
        self.signal_filter = Some(filter);
        self
    }

    /// Add backtest results.
    pub fn with_results(mut self, results: BacktestResults) -> Self {
        self.backtest_results = Some(results);
        self
    }

    /// Add parity vectors.
    pub fn with_parity(mut self, parity: ParityVectors) -> Self {
        self.parity_vectors = Some(parity);
        self
    }

    /// Add run fingerprint.
    pub fn with_fingerprint(mut self, fingerprint: String) -> Self {
        self.run_fingerprint = Some(fingerprint);
        self
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> ExportResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Save to file.
    pub fn save_to_file(&self, path: &Path) -> ExportResult<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Get a human-readable description.
    pub fn description(&self) -> String {
        let filter_str = self
            .signal_filter
            .as_ref()
            .map(|f| format!(" + {}", f.name))
            .unwrap_or_default();

        format!(
            "{} -> {} -> {}{}",
            self.signal_generator.name,
            self.position_manager.name,
            self.execution_model.name,
            filter_str
        )
    }
}

/// Component artifact (signal generator, position manager, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentArtifact {
    /// Component type name.
    pub name: String,
    /// Component parameters.
    pub parameters: HashMap<String, ParamArtifact>,
    /// Exit reference mode (for extreme-based exits).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_reference_mode: Option<ExitReferenceMode>,
}

impl ComponentArtifact {
    /// Create a new component artifact.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: HashMap::new(),
            exit_reference_mode: None,
        }
    }

    /// Add a parameter.
    pub fn param(mut self, name: impl Into<String>, value: ParamArtifact) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    /// Set exit reference mode.
    pub fn with_exit_mode(mut self, mode: ExitReferenceMode) -> Self {
        self.exit_reference_mode = Some(mode);
        self
    }

    /// Get an integer parameter, returning error if missing.
    pub fn get_int(&self, name: &str) -> ExportResult<i64> {
        match self.parameters.get(name) {
            Some(ParamArtifact::Int(v)) => Ok(*v),
            Some(ParamArtifact::Float(v)) => Ok(*v as i64),
            _ => Err(ExportError::MissingParam {
                component: self.name.clone(),
                param: name.to_string(),
            }),
        }
    }

    /// Get a float parameter, returning error if missing.
    pub fn get_float(&self, name: &str) -> ExportResult<f64> {
        match self.parameters.get(name) {
            Some(ParamArtifact::Float(v)) => Ok(*v),
            Some(ParamArtifact::Int(v)) => Ok(*v as f64),
            _ => Err(ExportError::MissingParam {
                component: self.name.clone(),
                param: name.to_string(),
            }),
        }
    }

    /// Get a boolean parameter, returning error if missing.
    pub fn get_bool(&self, name: &str) -> ExportResult<bool> {
        match self.parameters.get(name) {
            Some(ParamArtifact::Bool(v)) => Ok(*v),
            _ => Err(ExportError::MissingParam {
                component: self.name.clone(),
                param: name.to_string(),
            }),
        }
    }

    /// Get an integer parameter with default.
    pub fn get_int_or(&self, name: &str, default: i64) -> i64 {
        self.get_int(name).unwrap_or(default)
    }

    /// Get a float parameter with default.
    pub fn get_float_or(&self, name: &str, default: f64) -> f64 {
        self.get_float(name).unwrap_or(default)
    }

    /// Get a boolean parameter with default.
    pub fn get_bool_or(&self, name: &str, default: bool) -> bool {
        self.get_bool(name).unwrap_or(default)
    }
}

/// Parameter value in artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamArtifact {
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// String value.
    String(String),
}

impl From<i64> for ParamArtifact {
    fn from(v: i64) -> Self {
        ParamArtifact::Int(v)
    }
}

impl From<f64> for ParamArtifact {
    fn from(v: f64) -> Self {
        ParamArtifact::Float(v)
    }
}

impl From<bool> for ParamArtifact {
    fn from(v: bool) -> Self {
        ParamArtifact::Bool(v)
    }
}

impl From<String> for ParamArtifact {
    fn from(v: String) -> Self {
        ParamArtifact::String(v)
    }
}

impl From<&str> for ParamArtifact {
    fn from(v: &str) -> Self {
        ParamArtifact::String(v.to_string())
    }
}

/// Exit reference mode for extreme-based exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReferenceMode {
    /// Reference fixed at entry time.
    EntryFrozenReference,
    /// Trailing extreme tracked since entry.
    SinceEntryTrailingExtreme,
    /// Separate lookbacks for entry and exit.
    SeparateEntryExitLookbacks,
}

/// Backtest results embedded in artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResults {
    /// Symbol tested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Date range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    /// Performance metrics.
    pub metrics: BacktestMetrics,
}

/// Date range for backtest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date.
    pub start: NaiveDate,
    /// End date.
    pub end: NaiveDate,
}

/// Backtest performance metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacktestMetrics {
    /// Total return (e.g., 1.5 = 150%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_return: Option<f64>,
    /// Compound annual growth rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cagr: Option<f64>,
    /// Sharpe ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe: Option<f64>,
    /// Sortino ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortino: Option<f64>,
    /// Maximum drawdown (as positive decimal, e.g., 0.2 = 20%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_drawdown: Option<f64>,
    /// Win rate (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub win_rate: Option<f64>,
    /// Profit factor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_factor: Option<f64>,
    /// Total number of trades.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_trades: Option<usize>,
    /// Average bars held per trade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_bars_held: Option<f64>,
}

/// Parity test vectors for validating Pine Script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityVectors {
    /// Symbol used for test vectors.
    pub symbol: String,
    /// Entry points.
    #[serde(default)]
    pub entries: Vec<TradeEntry>,
    /// Exit points.
    #[serde(default)]
    pub exits: Vec<TradeExit>,
    /// Trade returns (for quick validation).
    #[serde(default)]
    pub trade_returns: Vec<f64>,
}

/// Trade entry for parity testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEntry {
    /// Entry date.
    pub date: NaiveDate,
    /// Entry price.
    pub price: f64,
    /// Signal value at entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_value: Option<f64>,
}

/// Trade exit for parity testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExit {
    /// Exit date.
    pub date: NaiveDate,
    /// Exit price.
    pub price: f64,
    /// Exit reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_creation() {
        let artifact = StrategyArtifact::new(
            "test-strategy".to_string(),
            ComponentArtifact::new("DonchianBreakout").param("lookback", 20.into()),
            ComponentArtifact::new("AtrTrailing").param("multiplier", 2.0.into()),
            ComponentArtifact::new("NextOpenFill"),
        );

        assert_eq!(artifact.strategy_id, "test-strategy");
        assert_eq!(artifact.signal_generator.name, "DonchianBreakout");
        assert_eq!(artifact.position_manager.name, "AtrTrailing");
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = StrategyArtifact::new(
            "test".to_string(),
            ComponentArtifact::new("Signal"),
            ComponentArtifact::new("PM"),
            ComponentArtifact::new("Exec"),
        );

        let json = artifact.to_json().unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Signal"));
    }

    #[test]
    fn test_component_params() {
        let comp = ComponentArtifact::new("Test")
            .param("int_val", 42.into())
            .param("float_val", 3.14.into())
            .param("bool_val", true.into());

        assert_eq!(comp.get_int("int_val").unwrap(), 42);
        assert!((comp.get_float("float_val").unwrap() - 3.14).abs() < 0.001);
        assert!(comp.get_bool("bool_val").unwrap());
    }

    #[test]
    fn test_param_defaults() {
        let comp = ComponentArtifact::new("Test");

        assert_eq!(comp.get_int_or("missing", 99), 99);
        assert!((comp.get_float_or("missing", 1.5) - 1.5).abs() < 0.001);
        assert!(!comp.get_bool_or("missing", false));
    }
}
