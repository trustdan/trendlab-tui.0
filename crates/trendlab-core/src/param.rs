//! Parameter definitions for Monte Carlo sampling.
//!
//! These types allow components to declare their tunable parameters
//! so the YOLO sampler can explore the parameter space.

use serde::{Deserialize, Serialize};

/// Parameter definition for Monte Carlo sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// Parameter name (unique within component)
    pub name: String,
    /// Parameter type with bounds
    pub param_type: ParamType,
    /// Human-readable description
    pub description: Option<String>,
}

impl ParamDef {
    /// Create an integer parameter definition.
    pub fn int(name: impl Into<String>, min: i64, max: i64, step: i64) -> Self {
        Self {
            name: name.into(),
            param_type: ParamType::Int { min, max, step },
            description: None,
        }
    }

    /// Create a float parameter definition.
    pub fn float(name: impl Into<String>, min: f64, max: f64, step: f64) -> Self {
        Self {
            name: name.into(),
            param_type: ParamType::Float { min, max, step },
            description: None,
        }
    }

    /// Create a boolean parameter definition.
    pub fn bool(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: ParamType::Bool,
            description: None,
        }
    }

    /// Create a choice parameter definition.
    pub fn choice(name: impl Into<String>, choices: Vec<String>) -> Self {
        Self {
            name: name.into(),
            param_type: ParamType::Choice(choices),
            description: None,
        }
    }

    /// Add a description to this parameter definition.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Count the number of possible values for this parameter.
    pub fn cardinality(&self) -> usize {
        self.param_type.cardinality()
    }
}

/// Parameter type with sampling bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// Integer parameter with min, max, and step
    Int {
        /// Minimum value (inclusive)
        min: i64,
        /// Maximum value (inclusive)
        max: i64,
        /// Step size
        step: i64,
    },
    /// Float parameter with min, max, and step
    Float {
        /// Minimum value (inclusive)
        min: f64,
        /// Maximum value (inclusive)
        max: f64,
        /// Step size
        step: f64,
    },
    /// Boolean parameter
    Bool,
    /// Choice from a set of options
    Choice(Vec<String>),
}

impl ParamType {
    /// Count the number of possible values for this parameter type.
    pub fn cardinality(&self) -> usize {
        match self {
            ParamType::Int { min, max, step } => {
                if *step <= 0 {
                    0
                } else {
                    ((max - min) / step + 1) as usize
                }
            }
            ParamType::Float { min, max, step } => {
                if *step <= 0.0 {
                    0
                } else {
                    ((max - min) / step + 1.0) as usize
                }
            }
            ParamType::Bool => 2,
            ParamType::Choice(choices) => choices.len(),
        }
    }
}

/// Concrete parameter value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// Choice value
    Choice(String),
}

impl ParamValue {
    /// Get as integer, if this is an Int variant.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ParamValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as float, if this is a Float variant.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ParamValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as bool, if this is a Bool variant.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as string, if this is a Choice variant.
    pub fn as_choice(&self) -> Option<&str> {
        match self {
            ParamValue::Choice(v) => Some(v),
            _ => None,
        }
    }
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamValue::Int(v) => write!(f, "{}", v),
            ParamValue::Float(v) => write!(f, "{:.4}", v),
            ParamValue::Bool(v) => write!(f, "{}", v),
            ParamValue::Choice(v) => write!(f, "{}", v),
        }
    }
}

/// A set of parameter values keyed by name.
pub type ParamSet = std::collections::HashMap<String, ParamValue>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_cardinality() {
        let def = ParamDef::int("lookback", 10, 50, 10);
        assert_eq!(def.cardinality(), 5); // 10, 20, 30, 40, 50
    }

    #[test]
    fn test_float_cardinality() {
        let def = ParamDef::float("multiplier", 1.0, 3.0, 0.5);
        assert_eq!(def.cardinality(), 5); // 1.0, 1.5, 2.0, 2.5, 3.0
    }

    #[test]
    fn test_bool_cardinality() {
        let def = ParamDef::bool("use_filter");
        assert_eq!(def.cardinality(), 2);
    }

    #[test]
    fn test_choice_cardinality() {
        let def = ParamDef::choice("ma_type", vec!["SMA".into(), "EMA".into(), "WMA".into()]);
        assert_eq!(def.cardinality(), 3);
    }

    #[test]
    fn test_param_value_accessors() {
        let v = ParamValue::Int(42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_float(), None);

        let v = ParamValue::Float(3.14);
        assert_eq!(v.as_float(), Some(3.14));
        assert_eq!(v.as_int(), None);
    }

    #[test]
    fn test_param_value_display() {
        assert_eq!(ParamValue::Int(42).to_string(), "42");
        assert_eq!(ParamValue::Float(3.1415).to_string(), "3.1415");
        assert_eq!(ParamValue::Bool(true).to_string(), "true");
        assert_eq!(ParamValue::Choice("SMA".into()).to_string(), "SMA");
    }
}
