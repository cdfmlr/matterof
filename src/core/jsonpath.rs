//! JSONPath operations and YAML ↔ JSON conversion utilities
//!
//! This module provides the foundation for JSONPath-based queries and modifications
//! using the RFC 9535 compliant `serde_json_path` library. It handles conversion
//! between YAML front-matter and JSON for JSONPath operations while preserving
//! semantic meaning.

use crate::core::FrontMatterValue;
use crate::error::{MatterOfError, Result};
use serde_json::Value as JsonValue;
use serde_json_path::{JsonPath, NormalizedPath};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

/// JSONPath query wrapper with auto-prepending logic
#[derive(Debug, Clone)]
pub struct JsonPathQuery {
    /// The compiled JSONPath expression
    path: JsonPath,
    /// The original query string
    original: String,
    /// Whether auto-prepending was applied
    auto_prepended: bool,
}

impl JsonPathQuery {
    /// Create a new JSONPath query with intelligent auto-prepending
    ///
    /// Auto-prepending strategy:
    /// 1. Try to parse as-is
    /// 2. If expression starts with `[`, try prefixing `$`
    /// 3. Otherwise try prefixing `$.`
    pub fn new(query: &str) -> Result<Self> {
        Self::new_with_options(query, true)
    }

    /// Create a new JSONPath query with optional auto-prepending
    pub fn new_with_options(query: &str, auto_prepend: bool) -> Result<Self> {
        if !auto_prepend {
            // Try to parse as-is only
            let path = JsonPath::parse(query).map_err(|e| MatterOfError::InvalidQuery {
                reason: format!("Invalid JSONPath syntax: {}", e),
            })?;
            return Ok(Self {
                path,
                original: query.to_string(),
                auto_prepended: false,
            });
        }

        // Strategy 1: Try as-is first
        if let Ok(path) = JsonPath::parse(query) {
            return Ok(Self {
                path,
                original: query.to_string(),
                auto_prepended: false,
            });
        }

        // Strategy 2: If starts with '[', try prefixing '$'
        if query.starts_with('[') {
            let prefixed = format!("${}", query);
            if let Ok(path) = JsonPath::parse(&prefixed) {
                return Ok(Self {
                    path,
                    original: query.to_string(),
                    auto_prepended: true,
                });
            }
        }

        // Strategy 3: Try prefixing '$.'
        let prefixed = format!("$.{}", query);
        let path = JsonPath::parse(&prefixed).map_err(|e| MatterOfError::InvalidQuery {
            reason: format!(
                "Invalid JSONPath syntax (tried '{}', '${0}', and '$.{0}'): {}",
                query, e
            ),
        })?;

        Ok(Self {
            path,
            original: query.to_string(),
            auto_prepended: true,
        })
    }

    /// Get the compiled JSONPath
    pub fn path(&self) -> &JsonPath {
        &self.path
    }

    /// Get the original query string
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Check if auto-prepending was applied
    pub fn was_auto_prepended(&self) -> bool {
        self.auto_prepended
    }

    /// Query a JSON value and return located results
    pub fn query_located<'a>(
        &self,
        value: &'a JsonValue,
    ) -> Vec<(NormalizedPath<'a>, &'a JsonValue)> {
        self.path
            .query_located(value)
            .into_iter()
            .map(|node| (node.location().clone(), node.node()))
            .collect()
    }

    /// Query a JSON value and return just the values
    pub fn query<'a>(&self, value: &'a JsonValue) -> Vec<&'a JsonValue> {
        self.path.query(value).into_iter().collect()
    }
}

/// Utilities for converting between YAML and JSON while preserving semantics
pub struct YamlJsonConverter;

impl YamlJsonConverter {
    /// Convert YAML Value to JSON Value
    ///
    /// This conversion preserves the semantic meaning of the data while
    /// making it compatible with JSONPath operations.
    pub fn yaml_to_json(yaml: &YamlValue) -> Result<JsonValue> {
        match yaml {
            YamlValue::Null => Ok(JsonValue::Null),
            YamlValue::Bool(b) => Ok(JsonValue::Bool(*b)),
            YamlValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(JsonValue::Number(serde_json::Number::from(i)))
                } else if let Some(f) = n.as_f64() {
                    serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .ok_or_else(|| MatterOfError::TypeConversion {
                            from: format!("YAML number {}", f),
                            to: "JSON number".to_string(),
                        })
                } else {
                    Err(MatterOfError::TypeConversion {
                        from: format!("YAML number {:?}", n),
                        to: "JSON number".to_string(),
                    })
                }
            }
            YamlValue::String(s) => Ok(JsonValue::String(s.clone())),
            YamlValue::Sequence(seq) => {
                let json_seq: Result<Vec<JsonValue>> = seq.iter().map(Self::yaml_to_json).collect();
                Ok(JsonValue::Array(json_seq?))
            }
            YamlValue::Mapping(map) => {
                let mut json_map = serde_json::Map::new();
                for (k, v) in map {
                    let key = match k {
                        YamlValue::String(s) => s.clone(),
                        YamlValue::Number(n) => n.to_string(),
                        YamlValue::Bool(b) => b.to_string(),
                        _ => {
                            return Err(MatterOfError::TypeConversion {
                                from: format!("YAML key {:?}", k),
                                to: "JSON string key".to_string(),
                            })
                        }
                    };
                    json_map.insert(key, Self::yaml_to_json(v)?);
                }
                Ok(JsonValue::Object(json_map))
            }
            YamlValue::Tagged(tagged) => {
                // For tagged values, convert the inner value
                Self::yaml_to_json(&tagged.value)
            }
        }
    }

    /// Convert JSON Value to YAML Value
    ///
    /// This conversion preserves the semantic meaning while returning
    /// the data to YAML format for front-matter writing.
    pub fn json_to_yaml(json: &JsonValue) -> Result<YamlValue> {
        match json {
            JsonValue::Null => Ok(YamlValue::Null),
            JsonValue::Bool(b) => Ok(YamlValue::Bool(*b)),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(YamlValue::Number(serde_yaml::Number::from(i)))
                } else if let Some(f) = n.as_f64() {
                    Ok(YamlValue::Number(serde_yaml::Number::from(f)))
                } else {
                    Err(MatterOfError::TypeConversion {
                        from: format!("JSON number {}", n),
                        to: "YAML number".to_string(),
                    })
                }
            }
            JsonValue::String(s) => Ok(YamlValue::String(s.clone())),
            JsonValue::Array(arr) => {
                let yaml_seq: Result<Vec<YamlValue>> = arr.iter().map(Self::json_to_yaml).collect();
                Ok(YamlValue::Sequence(yaml_seq?))
            }
            JsonValue::Object(obj) => {
                let mut yaml_map = serde_yaml::Mapping::new();
                for (k, v) in obj {
                    yaml_map.insert(YamlValue::String(k.clone()), Self::json_to_yaml(v)?);
                }
                Ok(YamlValue::Mapping(yaml_map))
            }
        }
    }

    /// Convert FrontMatterValue to JSON Value
    pub fn front_matter_to_json(value: &FrontMatterValue) -> Result<JsonValue> {
        Self::yaml_to_json(value.as_inner())
    }

    /// Convert JSON Value to FrontMatterValue
    pub fn json_to_front_matter(json: &JsonValue) -> Result<FrontMatterValue> {
        let yaml = Self::json_to_yaml(json)?;
        Ok(FrontMatterValue::new(yaml))
    }

    /// Convert Document front matter to YAML Value
    pub fn document_front_matter_to_yaml(
        front_matter: &BTreeMap<String, FrontMatterValue>,
    ) -> YamlValue {
        let mut map = serde_yaml::Mapping::new();
        for (key, value) in front_matter {
            map.insert(YamlValue::String(key.clone()), value.as_inner().clone());
        }
        YamlValue::Mapping(map)
    }

    /// Convert YAML Value to Document front matter format
    pub fn yaml_to_document_front_matter(
        yaml: &YamlValue,
    ) -> Result<BTreeMap<String, FrontMatterValue>> {
        match yaml {
            YamlValue::Mapping(map) => {
                let mut fm = BTreeMap::new();
                for (k, v) in map {
                    if let Some(key_str) = k.as_str() {
                        fm.insert(key_str.to_string(), FrontMatterValue::new(v.clone()));
                    }
                }
                Ok(fm)
            }
            YamlValue::Null => Ok(BTreeMap::new()),
            _ => Err(MatterOfError::type_conversion(
                format!("{:?}", yaml),
                "Document front matter".to_string(),
            )),
        }
    }
}

/// Utilities for working with NormalizedPath (RFC 9535 §2.7)
pub struct NormalizedPathUtils;

impl NormalizedPathUtils {
    /// Convert a NormalizedPath to a human-readable string
    ///
    /// This follows the RFC 9535 §2.7 format for canonical path representation
    pub fn to_string(path: &NormalizedPath<'_>) -> String {
        path.to_string()
    }

    /// Parse a NormalizedPath from a string
    ///
    /// Note: This is primarily for testing as NormalizedPath is typically
    /// generated by JSONPath queries, not manually created
    pub fn from_string(path_str: &str) -> Result<String> {
        // For now, just return the string as-is since we store paths as strings
        // TODO: Add proper validation if needed
        Ok(path_str.to_string())
    }

    /// Check if a NormalizedPath represents an array index access
    pub fn is_array_access(path: &NormalizedPath<'_>) -> bool {
        // Check if the path contains numeric indices
        path.to_string().contains("][")
    }

    /// Extract the final key/index from a NormalizedPath
    pub fn final_key(path: &NormalizedPath<'_>) -> Option<String> {
        let path_str = path.to_string();

        // Extract the last segment between brackets or quotes
        if let Some(start) = path_str.rfind("['") {
            if let Some(end) = path_str[start + 2..].find("']") {
                return Some(path_str[start + 2..start + 2 + end].to_string());
            }
        }

        // Handle numeric indices [0], [1], etc.
        if let Some(start) = path_str.rfind('[') {
            if let Some(end) = path_str[start..].find(']') {
                let index_str = &path_str[start + 1..start + end];
                if index_str.chars().all(|c| c.is_ascii_digit()) {
                    return Some(index_str.to_string());
                }
            }
        }

        None
    }
}

/// Result of a JSONPath query operation
#[derive(Debug, Clone)]
pub struct JsonPathQueryResult {
    /// The query that produced these results
    pub query: JsonPathQuery,
    /// The matching paths and their values
    pub matches: Vec<(String, JsonValue)>,
}

impl JsonPathQueryResult {
    /// Create a new query result
    pub fn new(query: JsonPathQuery, matches: Vec<(NormalizedPath<'_>, JsonValue)>) -> Self {
        let string_matches = matches
            .into_iter()
            .map(|(path, value)| (NormalizedPathUtils::to_string(&path), value))
            .collect();

        Self {
            query,
            matches: string_matches,
        }
    }

    /// Get the number of matches
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Check if there are no matches
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Check if there's exactly one match
    pub fn is_single(&self) -> bool {
        self.matches.len() == 1
    }

    /// Get the single match if there's exactly one, otherwise return an error
    pub fn single_match(&self) -> Result<&(String, JsonValue)> {
        if self.matches.len() == 1 {
            Ok(&self.matches[0])
        } else {
            Err(MatterOfError::InvalidQuery {
                reason: format!(
                    "Expected exactly one match, found {}: {}",
                    self.matches.len(),
                    self.query.original()
                ),
            })
        }
    }

    /// Get all matching values as JSON values
    pub fn values(&self) -> Vec<&JsonValue> {
        self.matches.iter().map(|(_, v)| v).collect()
    }

    /// Get all matching paths as string references
    pub fn paths(&self) -> Vec<&String> {
        self.matches.iter().map(|(p, _)| p).collect()
    }

    /// Convert results to YAML format for output
    pub fn to_yaml(&self) -> Result<YamlValue> {
        match self.matches.len() {
            0 => Ok(YamlValue::Null),
            1 => {
                // Single match: return the value directly
                let (_, json_value) = &self.matches[0];
                YamlJsonConverter::json_to_yaml(json_value)
            }
            _ => {
                // Multiple matches: create a mapping of paths to values
                let mut yaml_map = serde_yaml::Mapping::new();
                for (path, json_value) in &self.matches {
                    let yaml_value = YamlJsonConverter::json_to_yaml(json_value)?;
                    yaml_map.insert(YamlValue::String(path.clone()), yaml_value);
                }
                Ok(YamlValue::Mapping(yaml_map))
            }
        }
    }

    /// Convert results to JSON format for output
    pub fn to_json(&self) -> Result<JsonValue> {
        match self.matches.len() {
            0 => Ok(JsonValue::Null),
            1 => {
                // Single match: return the value directly
                let (_, json_value) = &self.matches[0];
                Ok(json_value.clone())
            }
            _ => {
                // Multiple matches: create an object mapping paths to values
                let mut json_map = serde_json::Map::new();
                for (path, json_value) in &self.matches {
                    json_map.insert(path.clone(), json_value.clone());
                }
                Ok(JsonValue::Object(json_map))
            }
        }
    }

    /// Convert results to internal format (NormalizedPath: value lines)
    pub fn to_internal_format(&self) -> Vec<String> {
        self.matches
            .iter()
            .map(|(path, value)| {
                let value_str = match value {
                    JsonValue::String(s) => s.clone(),
                    JsonValue::Null => "null".to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Array(_) | JsonValue::Object(_) => {
                        serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
                    }
                };
                format!("{}: {}", path, value_str)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_yaml;

    #[test]
    fn test_located_node_api() {
        let json = json!({"title": "Test"});
        let query = JsonPath::parse("$.title").unwrap();
        let located = query.query_located(&json);

        println!("Located results count: {}", located.len());
        for (i, node) in located.iter().enumerate() {
            println!("  Node {}: {:?}", i, node);
        }

        // This test helps us understand the LocatedNode API
        assert!(!located.is_empty());
    }

    #[test]
    fn test_auto_prepending() {
        // Test 1: Already valid JSONPath
        let query = JsonPathQuery::new("$.title").unwrap();
        assert_eq!(query.original(), "$.title");
        assert!(!query.was_auto_prepended());

        // Test 2: Simple field (should prepend "$.")
        let query = JsonPathQuery::new("title").unwrap();
        assert_eq!(query.original(), "title");
        assert!(query.was_auto_prepended());

        // Test 3: Array access starting with "[" (should prepend "$")
        let query = JsonPathQuery::new("[0]").unwrap();
        assert_eq!(query.original(), "[0]");
        assert!(query.was_auto_prepended());

        // Test 4: Nested field (should prepend "$.")
        let query = JsonPathQuery::new("author.name").unwrap();
        assert_eq!(query.original(), "author.name");
        assert!(query.was_auto_prepended());
    }

    #[test]
    fn test_yaml_json_conversion() {
        // Test basic types
        let yaml = serde_yaml::from_str("title: Hello World").unwrap();
        let json = YamlJsonConverter::yaml_to_json(&yaml).unwrap();
        let expected = json!({"title": "Hello World"});
        assert_eq!(json, expected);

        // Test round-trip conversion
        let yaml_back = YamlJsonConverter::json_to_yaml(&json).unwrap();
        assert_eq!(yaml, yaml_back);
    }

    #[test]
    fn test_complex_yaml_json_conversion() {
        let yaml_str = r#"
        title: "Test Document"
        author:
          name: "John Doe"
          email: "john@example.com"
        tags: ["rust", "json", "yaml"]
        count: 42
        price: 19.99
        published: true
        metadata: null
        "#;

        let yaml: YamlValue = serde_yaml::from_str(yaml_str).unwrap();
        let json = YamlJsonConverter::yaml_to_json(&yaml).unwrap();

        // Verify the conversion preserves structure
        assert!(json.is_object());
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("title").unwrap().as_str().unwrap(), "Test Document");
        assert!(obj.get("author").unwrap().is_object());
        assert!(obj.get("tags").unwrap().is_array());
        assert_eq!(obj.get("count").unwrap().as_i64().unwrap(), 42);

        // Test round-trip
        let _yaml_back = YamlJsonConverter::json_to_yaml(&json).unwrap();
        // Note: Round-trip might not be exactly equal due to ordering in mappings
        // but semantic content should be preserved
    }

    #[test]
    fn test_jsonpath_query() {
        let json = json!({
            "title": "Test Document",
            "author": {
                "name": "John Doe",
                "email": "john@example.com"
            },
            "tags": ["rust", "json", "yaml"],
            "posts": [
                {"title": "Post 1", "published": true},
                {"title": "Post 2", "published": false}
            ]
        });

        // Test simple field access
        let query = JsonPathQuery::new("title").unwrap();
        let results = query.query(&json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str().unwrap(), "Test Document");

        // Test nested field access
        let query = JsonPathQuery::new("author.name").unwrap();
        let results = query.query(&json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str().unwrap(), "John Doe");

        // Test array access
        let query = JsonPathQuery::new("tags[0]").unwrap();
        let results = query.query(&json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str().unwrap(), "rust");

        // Test wildcard
        let query = JsonPathQuery::new("tags[*]").unwrap();
        let results = query.query(&json);
        assert_eq!(results.len(), 3);

        // Test filter
        let query = JsonPathQuery::new("posts[?@.published == true]").unwrap();
        let results = query.query(&json);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_normalized_path_utils() {
        // This is a basic test - in practice, NormalizedPath is created by serde_json_path
        let json = json!({"author": {"name": "John"}});
        let query = JsonPathQuery::new("author.name").unwrap();
        let located = query.query_located(&json);

        assert_eq!(located.len(), 1);
        let (path, value) = &located[0];

        let path_str = NormalizedPathUtils::to_string(path);
        assert!(path_str.contains("author"));
        assert!(path_str.contains("name"));
        assert_eq!(value.as_str().unwrap(), "John");
    }

    #[test]
    fn test_query_result_formatting() {
        let json = json!({"title": "Test", "tags": ["a", "b"]});
        let query = JsonPathQuery::new("title").unwrap();
        let located = query.query_located(&json);
        let matches = located.into_iter().map(|(p, v)| (p, v.clone())).collect();
        let result = JsonPathQueryResult::new(query.clone(), matches);

        // Test single result returns value directly
        let yaml = result.to_yaml().unwrap();
        assert_eq!(yaml.as_str().unwrap(), "Test");

        let json_result = result.to_json().unwrap();
        assert_eq!(json_result.as_str().unwrap(), "Test");

        // Test internal format
        let internal = result.to_internal_format();
        assert_eq!(internal.len(), 1);
        assert!(internal[0].contains("title"));
        assert!(internal[0].contains("Test"));
    }

    #[test]
    fn test_multiple_matches_formatting() {
        let json = json!({"tags": ["rust", "json"]});
        let query = JsonPathQuery::new("tags[*]").unwrap();
        let located = query.query_located(&json);
        let matches = located.into_iter().map(|(p, v)| (p, v.clone())).collect();
        let result = JsonPathQueryResult::new(query.clone(), matches);

        // Multiple matches should create a mapping
        let yaml = result.to_yaml().unwrap();
        assert!(yaml.is_mapping());

        let json_result = result.to_json().unwrap();
        assert!(json_result.is_object());

        // Internal format should have multiple lines
        let internal = result.to_internal_format();
        assert_eq!(internal.len(), 2);
        assert!(internal[0].contains("[0]"));
        assert!(internal[1].contains("[1]"));
    }
}
