//! Core value types for front matter handling
//!
//! This module provides a clean abstraction over YAML values with type-safe
//! conversions and operations specific to front matter handling.

use crate::error::{MatterOfError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A type-safe wrapper around YAML values for front matter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrontMatterValue {
    inner: serde_yaml::Value,
}

impl FrontMatterValue {
    /// Create a new value from a YAML value
    pub fn new(value: serde_yaml::Value) -> Self {
        Self { inner: value }
    }

    /// Create a null value
    pub fn null() -> Self {
        Self::new(serde_yaml::Value::Null)
    }

    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Self::new(serde_yaml::Value::String(s.into()))
    }

    /// Create an integer value
    pub fn int(i: i64) -> Self {
        Self::new(serde_yaml::Value::Number(i.into()))
    }

    /// Create a float value
    pub fn float(f: f64) -> Self {
        Self::new(serde_yaml::Value::Number(serde_yaml::Number::from(f)))
    }

    /// Create a boolean value
    pub fn bool(b: bool) -> Self {
        Self::new(serde_yaml::Value::Bool(b))
    }

    /// Create an array value
    pub fn array(values: Vec<FrontMatterValue>) -> Self {
        let seq = values.into_iter().map(|v| v.inner).collect();
        Self::new(serde_yaml::Value::Sequence(seq))
    }

    /// Create an object value
    pub fn object(map: BTreeMap<String, FrontMatterValue>) -> Self {
        let mut yaml_map = serde_yaml::Mapping::new();
        for (k, v) in map {
            yaml_map.insert(serde_yaml::Value::String(k), v.inner);
        }
        Self::new(serde_yaml::Value::Mapping(yaml_map))
    }

    /// Get the inner YAML value
    pub fn into_inner(self) -> serde_yaml::Value {
        self.inner
    }

    /// Get a reference to the inner YAML value
    pub fn as_inner(&self) -> &serde_yaml::Value {
        &self.inner
    }

    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    /// Check if this value is a string
    pub fn is_string(&self) -> bool {
        self.inner.is_string()
    }

    /// Check if this value is a number
    pub fn is_number(&self) -> bool {
        self.inner.is_number()
    }

    /// Check if this value is a boolean
    pub fn is_bool(&self) -> bool {
        self.inner.is_bool()
    }

    /// Check if this value is an array
    pub fn is_array(&self) -> bool {
        self.inner.is_sequence()
    }

    /// Check if this value is an object
    pub fn is_object(&self) -> bool {
        self.inner.is_mapping()
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Option<&str> {
        self.inner.as_str()
    }

    /// Try to convert to integer
    pub fn as_int(&self) -> Option<i64> {
        self.inner.as_i64()
    }

    /// Try to convert to float
    pub fn as_float(&self) -> Option<f64> {
        self.inner.as_f64()
    }

    /// Try to convert to boolean
    pub fn as_bool(&self) -> Option<bool> {
        self.inner.as_bool()
    }

    /// Try to convert to array
    pub fn as_array(&self) -> Option<Vec<FrontMatterValue>> {
        self.inner.as_sequence().map(|seq| {
            seq.iter()
                .map(|v| FrontMatterValue::new(v.clone()))
                .collect()
        })
    }

    /// Try to convert to object
    pub fn as_object(&self) -> Option<BTreeMap<String, FrontMatterValue>> {
        self.inner.as_mapping().map(|map| {
            map.iter()
                .filter_map(|(k, v)| {
                    k.as_str()
                        .map(|key| (key.to_string(), FrontMatterValue::new(v.clone())))
                })
                .collect()
        })
    }

    /// Convert to string with fallback representations
    pub fn to_string_representation(&self) -> String {
        match &self.inner {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            serde_yaml::Value::Null => "null".to_string(),
            _ => serde_yaml::to_string(&self.inner)
                .unwrap_or_else(|_| "<invalid>".to_string())
                .trim()
                .to_string(),
        }
    }

    /// Parse from a string with type hint
    pub fn parse_from_string(s: &str, type_hint: Option<&ValueType>) -> Result<Self> {
        let trimmed = s.trim();

        match type_hint {
            Some(ValueType::String) => Ok(Self::string(s)),
            Some(ValueType::Int) => {
                let i = trimmed
                    .parse::<i64>()
                    .map_err(|_| MatterOfError::type_conversion(s, "integer"))?;
                Ok(Self::int(i))
            }
            Some(ValueType::Float) => {
                let f = trimmed
                    .parse::<f64>()
                    .map_err(|_| MatterOfError::type_conversion(s, "float"))?;
                Ok(Self::float(f))
            }
            Some(ValueType::Bool) => {
                let b = match trimmed.to_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => true,
                    "false" | "no" | "off" | "0" => false,
                    _ => return Err(MatterOfError::type_conversion(s, "boolean")),
                };
                Ok(Self::bool(b))
            }
            Some(ValueType::Array) => {
                // Simple comma-separated parsing for CLI convenience
                let values: Result<Vec<_>> = s
                    .split(',')
                    .map(|part| Self::parse_from_string(part.trim(), None))
                    .collect();
                Ok(Self::array(values?))
            }
            Some(ValueType::Object) => {
                // Try to parse as YAML for objects
                let yaml_val: serde_yaml::Value = serde_yaml::from_str(s)?;
                Ok(Self::new(yaml_val))
            }
            None => {
                // Auto-detect type
                if let Ok(i) = trimmed.parse::<i64>() {
                    Ok(Self::int(i))
                } else if let Ok(f) = trimmed.parse::<f64>() {
                    Ok(Self::float(f))
                } else if let Ok(b) = trimmed.parse::<bool>() {
                    Ok(Self::bool(b))
                } else {
                    Ok(Self::string(s))
                }
            }
        }
    }

    /// Deep merge with another value
    pub fn merge(&mut self, other: FrontMatterValue) -> Result<()> {
        self.inner = merge_yaml_values(self.inner.clone(), other.inner)?;
        Ok(())
    }
}

/// Supported value types for type conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Int,
    Float,
    Bool,
    Array,
    Object,
}

impl ValueType {
    /// Parse a `ValueType` from its string name (e.g. `"string"`, `"int"`, `"bool"`)
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "string" | "str" => Some(Self::String),
            "int" | "integer" | "i64" => Some(Self::Int),
            "float" | "f64" | "number" => Some(Self::Float),
            "bool" | "boolean" => Some(Self::Bool),
            "array" | "list" | "sequence" => Some(Self::Array),
            "object" | "map" | "mapping" => Some(Self::Object),
            _ => None,
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Int => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Bool => write!(f, "bool"),
            Self::Array => write!(f, "array"),
            Self::Object => write!(f, "object"),
        }
    }
}

impl From<serde_yaml::Value> for FrontMatterValue {
    fn from(value: serde_yaml::Value) -> Self {
        Self::new(value)
    }
}

impl From<FrontMatterValue> for serde_yaml::Value {
    fn from(value: FrontMatterValue) -> Self {
        value.inner
    }
}

impl fmt::Display for FrontMatterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_representation())
    }
}

/// Deep merge two YAML values
fn merge_yaml_values(
    mut target: serde_yaml::Value,
    source: serde_yaml::Value,
) -> Result<serde_yaml::Value> {
    match (&mut target, source) {
        (serde_yaml::Value::Mapping(target_map), serde_yaml::Value::Mapping(source_map)) => {
            for (key, value) in source_map {
                if let Some(existing) = target_map.get_mut(&key) {
                    *existing = merge_yaml_values(existing.clone(), value)?;
                } else {
                    target_map.insert(key, value);
                }
            }
            Ok(target)
        }
        (serde_yaml::Value::Sequence(target_seq), serde_yaml::Value::Sequence(source_seq)) => {
            target_seq.extend(source_seq);
            Ok(target)
        }
        // For non-mergeable types, source overwrites target
        (_, source) => Ok(source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        let str_val = FrontMatterValue::string("hello");
        assert!(str_val.is_string());
        assert_eq!(str_val.as_string(), Some("hello"));

        let int_val = FrontMatterValue::int(42);
        assert!(int_val.is_number());
        assert_eq!(int_val.as_int(), Some(42));

        let bool_val = FrontMatterValue::bool(true);
        assert!(bool_val.is_bool());
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_string_parsing() {
        let val = FrontMatterValue::parse_from_string("42", Some(&ValueType::Int)).unwrap();
        assert_eq!(val.as_int(), Some(42));

        let val = FrontMatterValue::parse_from_string("true", Some(&ValueType::Bool)).unwrap();
        assert_eq!(val.as_bool(), Some(true));

        let val = FrontMatterValue::parse_from_string("3.14", Some(&ValueType::Float)).unwrap();
        assert_eq!(val.as_float(), Some(3.14));
    }

    #[test]
    fn test_auto_type_detection() {
        let val = FrontMatterValue::parse_from_string("42", None).unwrap();
        assert_eq!(val.as_int(), Some(42));

        let val = FrontMatterValue::parse_from_string("true", None).unwrap();
        assert_eq!(val.as_bool(), Some(true));

        let val = FrontMatterValue::parse_from_string("hello", None).unwrap();
        assert_eq!(val.as_string(), Some("hello"));
    }

    #[test]
    fn test_value_merge() {
        let mut obj1 = FrontMatterValue::object({
            let mut map = BTreeMap::new();
            map.insert("a".to_string(), FrontMatterValue::int(1));
            map.insert("b".to_string(), FrontMatterValue::string("hello"));
            map
        });

        let obj2 = FrontMatterValue::object({
            let mut map = BTreeMap::new();
            map.insert("b".to_string(), FrontMatterValue::string("world"));
            map.insert("c".to_string(), FrontMatterValue::int(3));
            map
        });

        obj1.merge(obj2).unwrap();

        let result = obj1.as_object().unwrap();
        assert_eq!(result.get("a").unwrap().as_int(), Some(1));
        assert_eq!(result.get("b").unwrap().as_string(), Some("world"));
        assert_eq!(result.get("c").unwrap().as_int(), Some(3));
    }

    #[test]
    fn test_array_operations() {
        let arr = FrontMatterValue::array(vec![
            FrontMatterValue::int(1),
            FrontMatterValue::string("hello"),
            FrontMatterValue::bool(true),
        ]);

        assert!(arr.is_array());
        let values = arr.as_array().unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0].as_int(), Some(1));
        assert_eq!(values[1].as_string(), Some("hello"));
        assert_eq!(values[2].as_bool(), Some(true));
    }
}
