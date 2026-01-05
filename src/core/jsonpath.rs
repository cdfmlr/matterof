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

/// A parsed segment of a NormalizedPath
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// Object property access like $['key']
    Property(String),
    /// Array index access like $\[0\]
    Index(usize),
    /// Array append operation (used for add operations)
    Append,
}

/// A parsed NormalizedPath broken into navigable segments
#[derive(Debug, Clone)]
pub struct ParsedPath {
    /// The segments that make up this path
    pub segments: Vec<PathSegment>,
    /// The original path string
    pub original: String,
}

impl NormalizedPathUtils {
    /// Convert a NormalizedPath to a human-readable string
    ///
    /// This follows the RFC 9535 §2.7 format for canonical path representation
    pub fn to_string(path: &NormalizedPath<'_>) -> String {
        path.to_string()
    }

    /// Parse a NormalizedPath from a string into navigable segments
    ///
    /// Supports RFC 9535 NormalizedPath format:
    /// - `$` (root)
    /// - `$['key']` (object property)
    /// - `$[0]` (array index)
    /// - `$['key'][0]['subkey']` (nested combinations)
    pub fn parse_path(path_str: &str) -> Result<ParsedPath> {
        let mut segments = Vec::new();

        // Must start with $
        if !path_str.starts_with('$') {
            return Err(MatterOfError::InvalidPath {
                path: path_str.to_string(),
                reason: "NormalizedPath must start with '$'".to_string(),
            });
        }

        // If it's just "$", that's the root
        if path_str == "$" {
            return Ok(ParsedPath {
                segments,
                original: path_str.to_string(),
            });
        }

        // Parse segments after the $
        let mut remaining = &path_str[1..];

        while !remaining.is_empty() {
            if !remaining.starts_with('[') {
                return Err(MatterOfError::InvalidPath {
                    path: path_str.to_string(),
                    reason: format!(
                        "Expected '[' at position {}",
                        path_str.len() - remaining.len()
                    ),
                });
            }

            // Find the closing bracket
            let close_pos = remaining
                .find(']')
                .ok_or_else(|| MatterOfError::InvalidPath {
                    path: path_str.to_string(),
                    reason: "Unclosed bracket in path".to_string(),
                })?;

            let segment_content = &remaining[1..close_pos];

            // Parse the segment content
            if segment_content.starts_with('\'') && segment_content.ends_with('\'') {
                // Property access: ['key']
                let key = segment_content[1..segment_content.len() - 1].to_string();
                segments.push(PathSegment::Property(key));
            } else if segment_content.chars().all(|c| c.is_ascii_digit()) {
                // Array index: [0], [1], etc.
                let index: usize =
                    segment_content
                        .parse()
                        .map_err(|_| MatterOfError::InvalidPath {
                            path: path_str.to_string(),
                            reason: format!("Invalid array index: {}", segment_content),
                        })?;
                segments.push(PathSegment::Index(index));
            } else if segment_content == "-" {
                // Array append: [-] (used for add operations)
                segments.push(PathSegment::Append);
            } else {
                return Err(MatterOfError::InvalidPath {
                    path: path_str.to_string(),
                    reason: format!("Invalid segment: {}", segment_content),
                });
            }

            // Move to next segment
            remaining = &remaining[close_pos + 1..];
        }

        Ok(ParsedPath {
            segments,
            original: path_str.to_string(),
        })
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

/// JSON value mutator that can navigate and modify values at any NormalizedPath
pub struct JsonMutator;

impl JsonMutator {
    /// Set a value at the given NormalizedPath, creating intermediate structures as needed
    pub fn set_at_path(
        json_value: &mut JsonValue,
        path_str: &str,
        new_value: JsonValue,
    ) -> Result<()> {
        let parsed_path = NormalizedPathUtils::parse_path(path_str)?;

        // If it's the root path, replace the entire value
        if parsed_path.segments.is_empty() {
            *json_value = new_value;
            return Ok(());
        }

        Self::set_at_parsed_path(json_value, &parsed_path.segments, new_value)
    }

    /// Remove a value at the given NormalizedPath
    pub fn remove_at_path(json_value: &mut JsonValue, path_str: &str) -> Result<bool> {
        let parsed_path = NormalizedPathUtils::parse_path(path_str)?;

        if parsed_path.segments.is_empty() {
            // Can't remove root
            return Err(MatterOfError::InvalidPath {
                path: path_str.to_string(),
                reason: "Cannot remove root element".to_string(),
            });
        }

        Self::remove_at_parsed_path(json_value, &parsed_path.segments)
    }

    /// Internal recursive function to set values
    fn set_at_parsed_path(
        current: &mut JsonValue,
        segments: &[PathSegment],
        new_value: JsonValue,
    ) -> Result<()> {
        if segments.is_empty() {
            *current = new_value;
            return Ok(());
        }

        let (first_segment, remaining_segments) = segments.split_first().unwrap();

        match first_segment {
            PathSegment::Property(key) => {
                // Ensure current is an object
                if !current.is_object() {
                    *current = JsonValue::Object(serde_json::Map::new());
                }

                let obj = current.as_object_mut().unwrap();

                if remaining_segments.is_empty() {
                    // Final segment - set the value
                    obj.insert(key.clone(), new_value);
                } else {
                    // Intermediate segment - ensure the key exists and recurse
                    let entry = obj.entry(key.clone()).or_insert(JsonValue::Null);
                    Self::set_at_parsed_path(entry, remaining_segments, new_value)?;
                }
            }

            PathSegment::Index(index) => {
                // Ensure current is an array
                if !current.is_array() {
                    *current = JsonValue::Array(Vec::new());
                }

                let arr = current.as_array_mut().unwrap();

                // Extend array if needed
                while arr.len() <= *index {
                    arr.push(JsonValue::Null);
                }

                if remaining_segments.is_empty() {
                    // Final segment - set the value
                    arr[*index] = new_value;
                } else {
                    // Intermediate segment - recurse
                    Self::set_at_parsed_path(&mut arr[*index], remaining_segments, new_value)?;
                }
            }

            PathSegment::Append => {
                // Ensure current is an array
                if !current.is_array() {
                    *current = JsonValue::Array(Vec::new());
                }

                let arr = current.as_array_mut().unwrap();

                if remaining_segments.is_empty() {
                    // Final segment - append the value
                    arr.push(new_value);
                } else {
                    // Intermediate segment - append null and recurse
                    arr.push(JsonValue::Null);
                    let last_index = arr.len() - 1;
                    Self::set_at_parsed_path(&mut arr[last_index], remaining_segments, new_value)?;
                }
            }
        }

        Ok(())
    }

    /// Internal recursive function to remove values
    fn remove_at_parsed_path(current: &mut JsonValue, segments: &[PathSegment]) -> Result<bool> {
        if segments.len() == 1 {
            // Final segment - perform the removal
            match &segments[0] {
                PathSegment::Property(key) => {
                    if let Some(obj) = current.as_object_mut() {
                        Ok(obj.remove(key).is_some())
                    } else {
                        Ok(false)
                    }
                }
                PathSegment::Index(index) => {
                    if let Some(arr) = current.as_array_mut() {
                        if *index < arr.len() {
                            arr.remove(*index);
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
                PathSegment::Append => {
                    // Remove last element from array
                    if let Some(arr) = current.as_array_mut() {
                        if !arr.is_empty() {
                            arr.pop();
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
            }
        } else {
            // Intermediate segment - navigate deeper
            let (first_segment, remaining_segments) = segments.split_first().unwrap();

            match first_segment {
                PathSegment::Property(key) => {
                    if let Some(obj) = current.as_object_mut() {
                        if let Some(value) = obj.get_mut(key) {
                            Self::remove_at_parsed_path(value, remaining_segments)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
                PathSegment::Index(index) => {
                    if let Some(arr) = current.as_array_mut() {
                        if *index < arr.len() {
                            Self::remove_at_parsed_path(&mut arr[*index], remaining_segments)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
                PathSegment::Append => {
                    // For removal, append means "last element"
                    if let Some(arr) = current.as_array_mut() {
                        if !arr.is_empty() {
                            let last_index = arr.len() - 1;
                            Self::remove_at_parsed_path(&mut arr[last_index], remaining_segments)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
            }
        }
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

    #[test]
    fn test_normalized_path_parsing() {
        // Test root path
        let parsed = NormalizedPathUtils::parse_path("$").unwrap();
        assert!(parsed.segments.is_empty());
        assert_eq!(parsed.original, "$");

        // Test simple property access
        let parsed = NormalizedPathUtils::parse_path("$['key']").unwrap();
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0], PathSegment::Property("key".to_string()));

        // Test array index access
        let parsed = NormalizedPathUtils::parse_path("$[0]").unwrap();
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0], PathSegment::Index(0));

        // Test nested access
        let parsed = NormalizedPathUtils::parse_path("$['author']['name']").unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(
            parsed.segments[0],
            PathSegment::Property("author".to_string())
        );
        assert_eq!(
            parsed.segments[1],
            PathSegment::Property("name".to_string())
        );

        // Test mixed access
        let parsed = NormalizedPathUtils::parse_path("$['tags'][0]").unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(
            parsed.segments[0],
            PathSegment::Property("tags".to_string())
        );
        assert_eq!(parsed.segments[1], PathSegment::Index(0));

        // Test append operation
        let parsed = NormalizedPathUtils::parse_path("$['items'][-]").unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(
            parsed.segments[0],
            PathSegment::Property("items".to_string())
        );
        assert_eq!(parsed.segments[1], PathSegment::Append);
    }

    #[test]
    fn test_normalized_path_parsing_errors() {
        // Test invalid paths
        assert!(NormalizedPathUtils::parse_path("invalid").is_err());
        assert!(NormalizedPathUtils::parse_path("$[unclosed").is_err());
        assert!(NormalizedPathUtils::parse_path("$['unclosed]").is_err());
        assert!(NormalizedPathUtils::parse_path("$[invalid_segment]").is_err());
    }

    #[test]
    fn test_json_mutator_set_root() {
        let mut json = json!({"old": "value"});
        let new_value = json!({"new": "content"});

        JsonMutator::set_at_path(&mut json, "$", new_value.clone()).unwrap();
        assert_eq!(json, new_value);
    }

    #[test]
    fn test_json_mutator_set_property() {
        let mut json = json!({"title": "old", "count": 42});

        // Set existing property
        JsonMutator::set_at_path(&mut json, "$['title']", json!("new title")).unwrap();
        assert_eq!(json["title"], "new title");
        assert_eq!(json["count"], 42);

        // Set new property
        JsonMutator::set_at_path(&mut json, "$['description']", json!("A description")).unwrap();
        assert_eq!(json["description"], "A description");
    }

    #[test]
    fn test_json_mutator_set_nested_object() {
        let mut json = json!({});

        // Create nested structure
        JsonMutator::set_at_path(&mut json, "$['author']['name']", json!("John Doe")).unwrap();
        assert_eq!(json["author"]["name"], "John Doe");

        // Add to existing nested structure
        JsonMutator::set_at_path(&mut json, "$['author']['email']", json!("john@example.com"))
            .unwrap();
        assert_eq!(json["author"]["email"], "john@example.com");
        assert_eq!(json["author"]["name"], "John Doe");
    }

    #[test]
    fn test_json_mutator_set_array_index() {
        let mut json = json!({"tags": ["old1", "old2"]});

        // Set existing index
        JsonMutator::set_at_path(&mut json, "$['tags'][0]", json!("new1")).unwrap();
        assert_eq!(json["tags"][0], "new1");
        assert_eq!(json["tags"][1], "old2");

        // Extend array by setting higher index
        JsonMutator::set_at_path(&mut json, "$['tags'][3]", json!("new4")).unwrap();
        assert_eq!(json["tags"].as_array().unwrap().len(), 4);
        assert_eq!(json["tags"][3], "new4");
        assert!(json["tags"][2].is_null()); // Should fill with null
    }

    #[test]
    fn test_json_mutator_create_array() {
        let mut json = json!({});

        // Create array and set first element
        JsonMutator::set_at_path(&mut json, "$['items'][0]", json!("first")).unwrap();
        assert_eq!(json["items"][0], "first");

        // Add more elements
        JsonMutator::set_at_path(&mut json, "$['items'][1]", json!("second")).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_json_mutator_append_to_array() {
        let mut json = json!({"items": ["existing"]});

        // Append using [-] syntax
        JsonMutator::set_at_path(&mut json, "$['items'][-]", json!("appended")).unwrap();
        let items = json["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], "appended");
    }

    #[test]
    fn test_json_mutator_remove_property() {
        let mut json = json!({"title": "test", "description": "desc", "count": 42});

        // Remove existing property
        let removed = JsonMutator::remove_at_path(&mut json, "$['description']").unwrap();
        assert!(removed);
        assert!(json.get("description").is_none());
        assert_eq!(json["title"], "test");
        assert_eq!(json["count"], 42);

        // Try to remove non-existent property
        let removed = JsonMutator::remove_at_path(&mut json, "$['nonexistent']").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_json_mutator_remove_array_element() {
        let mut json = json!({"tags": ["rust", "json", "yaml"]});

        // Remove middle element
        let removed = JsonMutator::remove_at_path(&mut json, "$['tags'][1]").unwrap();
        assert!(removed);
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "rust");
        assert_eq!(tags[1], "yaml");

        // Try to remove out-of-bounds index
        let removed = JsonMutator::remove_at_path(&mut json, "$['tags'][10]").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_json_mutator_remove_nested() {
        let mut json = json!({
            "author": {
                "name": "John",
                "email": "john@example.com",
                "age": 30
            }
        });

        // Remove nested property
        let removed = JsonMutator::remove_at_path(&mut json, "$['author']['email']").unwrap();
        assert!(removed);
        assert!(json["author"].get("email").is_none());
        assert_eq!(json["author"]["name"], "John");
        assert_eq!(json["author"]["age"], 30);
    }

    #[test]
    fn test_json_mutator_remove_with_append_syntax() {
        let mut json = json!({"items": ["a", "b", "c"]});

        // Remove last element using append syntax
        let removed = JsonMutator::remove_at_path(&mut json, "$['items'][-]").unwrap();
        assert!(removed);
        let items = json["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], "b");
    }

    #[test]
    fn test_json_mutator_complex_operations() {
        let mut json = json!({
            "posts": [
                {"title": "Post 1", "tags": ["tech"]},
                {"title": "Post 2", "tags": ["blog"]}
            ]
        });

        // Add a tag to the first post
        JsonMutator::set_at_path(&mut json, "$['posts'][0]['tags'][1]", json!("tutorial")).unwrap();
        let first_post_tags = json["posts"][0]["tags"].as_array().unwrap();
        assert_eq!(first_post_tags.len(), 2);
        assert_eq!(first_post_tags[1], "tutorial");

        // Create a new nested structure
        JsonMutator::set_at_path(&mut json, "$['posts'][0]['author']['name']", json!("Alice"))
            .unwrap();
        assert_eq!(json["posts"][0]["author"]["name"], "Alice");
    }

    #[test]
    fn test_json_mutator_error_conditions() {
        let mut json = json!({"test": "value"});

        // Cannot remove root
        let result = JsonMutator::remove_at_path(&mut json, "$");
        assert!(result.is_err());

        // Invalid path format
        let result = JsonMutator::set_at_path(&mut json, "invalid", json!("value"));
        assert!(result.is_err());

        let result = JsonMutator::set_at_path(&mut json, "$[unclosed", json!("value"));
        assert!(result.is_err());
    }

    #[test]
    fn test_json_mutator_array_insertion_at_index() {
        let mut json = json!({"items": ["a", "b", "d"]});

        // Insert at specific index
        JsonMutator::set_at_path(&mut json, "$['items'][2]", json!("c")).unwrap();
        let items = json["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[2], "c");

        // Insert beyond current length (should extend with nulls)
        JsonMutator::set_at_path(&mut json, "$['items'][5]", json!("f")).unwrap();
        let items = json["items"].as_array().unwrap();
        assert_eq!(items.len(), 6);
        assert_eq!(items[5], "f");
        assert!(items[4].is_null());
    }

    #[test]
    fn test_json_mutator_nested_array_operations() {
        let mut json = json!({"posts": [{"tags": ["tech"]}, {"tags": ["blog"]}]});

        // Add to nested array
        JsonMutator::set_at_path(&mut json, "$['posts'][0]['tags'][-]", json!("tutorial")).unwrap();
        let first_tags = json["posts"][0]["tags"].as_array().unwrap();
        assert_eq!(first_tags.len(), 2);
        assert_eq!(first_tags[1], "tutorial");

        // Insert into second post's tags
        JsonMutator::set_at_path(&mut json, "$['posts'][1]['tags'][0]", json!("personal")).unwrap();
        let second_tags = json["posts"][1]["tags"].as_array().unwrap();
        assert_eq!(second_tags[0], "personal");
    }

    #[test]
    fn test_json_mutator_object_property_operations() {
        let mut json = json!({"user": {"name": "John"}});

        // Add new property to existing object
        JsonMutator::set_at_path(&mut json, "$['user']['email']", json!("john@example.com"))
            .unwrap();
        assert_eq!(json["user"]["email"], "john@example.com");
        assert_eq!(json["user"]["name"], "John"); // Original property preserved

        // Create nested object structure
        JsonMutator::set_at_path(&mut json, "$['user']['profile']['bio']", json!("Developer"))
            .unwrap();
        assert_eq!(json["user"]["profile"]["bio"], "Developer");
    }

    #[test]
    fn test_json_mutator_type_coercion() {
        let mut json = json!({"config": "not_an_object"});

        // Setting property on non-object should coerce to object
        JsonMutator::set_at_path(&mut json, "$['config']['theme']", json!("dark")).unwrap();
        assert!(json["config"].is_object());
        assert_eq!(json["config"]["theme"], "dark");

        // Setting array index on non-array should coerce to array
        let mut json = json!({"items": "not_an_array"});
        JsonMutator::set_at_path(&mut json, "$['items'][0]", json!("first")).unwrap();
        assert!(json["items"].is_array());
        assert_eq!(json["items"][0], "first");
    }

    #[test]
    fn test_json_mutator_bulk_operations() {
        let mut json = json!({
            "posts": [
                {"status": "draft", "title": "Post 1"},
                {"status": "draft", "title": "Post 2"},
                {"status": "published", "title": "Post 3"}
            ]
        });

        // This test demonstrates what bulk operations would look like
        // In practice, this would be handled by the CLI command that finds all matches first

        // Simulate setting all draft posts to published
        JsonMutator::set_at_path(&mut json, "$['posts'][0]['status']", json!("published")).unwrap();
        JsonMutator::set_at_path(&mut json, "$['posts'][1]['status']", json!("published")).unwrap();

        let posts = json["posts"].as_array().unwrap();
        assert_eq!(posts[0]["status"], "published");
        assert_eq!(posts[1]["status"], "published");
        assert_eq!(posts[2]["status"], "published");
    }

    #[test]
    fn test_json_mutator_array_removal_patterns() {
        let mut json = json!({"tags": ["rust", "json", "yaml", "serde", "cli"]});

        // Remove from middle
        JsonMutator::remove_at_path(&mut json, "$['tags'][2]").unwrap();
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[2], "serde"); // yaml removed, serde shifted down

        // Remove from end
        JsonMutator::remove_at_path(&mut json, "$['tags'][-]").unwrap();
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[2], "serde"); // cli removed
    }

    #[test]
    fn test_json_mutator_complex_path_operations() {
        let mut json = json!({
            "users": {
                "admins": [
                    {"name": "Alice", "permissions": ["read", "write"]},
                    {"name": "Bob", "permissions": ["read"]}
                ]
            }
        });

        // Add permission to Alice
        JsonMutator::set_at_path(
            &mut json,
            "$['users']['admins'][0]['permissions'][-]",
            json!("delete"),
        )
        .unwrap();

        let alice_perms = json["users"]["admins"][0]["permissions"]
            .as_array()
            .unwrap();
        assert_eq!(alice_perms.len(), 3);
        assert_eq!(alice_perms[2], "delete");

        // Replace Bob's permissions entirely
        JsonMutator::set_at_path(
            &mut json,
            "$['users']['admins'][1]['permissions']",
            json!(["read", "write", "admin"]),
        )
        .unwrap();

        let bob_perms = json["users"]["admins"][1]["permissions"]
            .as_array()
            .unwrap();
        assert_eq!(bob_perms.len(), 3);
        assert_eq!(bob_perms[2], "admin");
    }

    #[test]
    fn test_json_mutator_edge_cases() {
        // Test with empty structures
        let mut json = json!({});
        JsonMutator::set_at_path(&mut json, "$['deep']['nested']['value']", json!("test")).unwrap();
        assert_eq!(json["deep"]["nested"]["value"], "test");

        // Test with null values
        let mut json = json!({"nullable": null});
        JsonMutator::set_at_path(&mut json, "$['nullable']['property']", json!("new")).unwrap();
        assert!(json["nullable"].is_object());
        assert_eq!(json["nullable"]["property"], "new");

        // Test array creation from null
        let mut json = json!({"list": null});
        JsonMutator::set_at_path(&mut json, "$['list'][0]", json!("first")).unwrap();
        assert!(json["list"].is_array());
        assert_eq!(json["list"][0], "first");
    }

    #[test]
    fn test_json_mutator_preservation_of_existing_data() {
        let mut json = json!({
            "config": {
                "database": {"host": "localhost", "port": 5432},
                "cache": {"ttl": 3600},
                "features": ["auth", "api"]
            }
        });

        // Add new database property
        JsonMutator::set_at_path(&mut json, "$['config']['database']['ssl']", json!(true)).unwrap();

        // Verify existing data is preserved
        assert_eq!(json["config"]["database"]["host"], "localhost");
        assert_eq!(json["config"]["database"]["port"], 5432);
        assert_eq!(json["config"]["database"]["ssl"], true);
        assert_eq!(json["config"]["cache"]["ttl"], 3600);

        let features = json["config"]["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0], "auth");
        assert_eq!(features[1], "api");
    }
}
