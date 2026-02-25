//! Composable query system for filtering and selecting front matter
//!
//! This module provides a flexible query builder system that allows for
//! complex filtering and selection of front matter keys and values.

use crate::core::{path::KeyPath, value::FrontMatterValue};
use crate::error::Result;
use regex::Regex;
use std::collections::BTreeMap;

/// Function type used in `QueryCondition::Custom`
type QueryPredicate = dyn Fn(&KeyPath, &FrontMatterValue) -> bool + Send + Sync;

/// A query builder for selecting front matter data
pub struct Query {
    conditions: Vec<QueryCondition>,
    combine_mode: CombineMode,
}

/// How multiple query conditions should be combined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineMode {
    /// All conditions must match (AND)
    All,
    /// Any condition can match (OR)
    Any,
}

/// Individual query conditions
pub enum QueryCondition {
    /// Select all keys
    All,
    /// Match specific key paths (hierarchical matching)
    KeyPaths(Vec<KeyPath>),
    /// Match specific key paths exactly (no hierarchical matching)
    ExactKeyPaths(Vec<KeyPath>),
    /// Match keys using regex
    KeyRegex(Regex),
    /// Match values exactly
    ValueExact(FrontMatterValue),
    /// Match values using regex (converted to string)
    ValueRegex(Regex),
    /// Match keys at a specific depth
    Depth(usize),
    /// Match keys that exist (not null/missing)
    Exists,
    /// Match keys that are null or missing
    Missing,
    /// Match values by type
    ValueType(ValueTypeCondition),
    /// Custom predicate function
    Custom(Box<QueryPredicate>),
}

impl std::fmt::Debug for QueryCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::KeyPaths(paths) => f.debug_tuple("KeyPaths").field(paths).finish(),
            Self::ExactKeyPaths(paths) => f.debug_tuple("ExactKeyPaths").field(paths).finish(),
            Self::KeyRegex(regex) => f.debug_tuple("KeyRegex").field(regex).finish(),
            Self::ValueExact(value) => f.debug_tuple("ValueExact").field(value).finish(),
            Self::ValueRegex(regex) => f.debug_tuple("ValueRegex").field(regex).finish(),
            Self::Depth(depth) => f.debug_tuple("Depth").field(depth).finish(),
            Self::Exists => write!(f, "Exists"),
            Self::Missing => write!(f, "Missing"),
            Self::ValueType(vt) => f.debug_tuple("ValueType").field(vt).finish(),
            Self::Custom(_) => write!(f, "Custom(<function>)"),
        }
    }
}

impl Clone for QueryCondition {
    fn clone(&self) -> Self {
        match self {
            Self::All => Self::All,
            Self::KeyPaths(paths) => Self::KeyPaths(paths.clone()),
            Self::ExactKeyPaths(paths) => Self::ExactKeyPaths(paths.clone()),
            Self::KeyRegex(regex) => Self::KeyRegex(regex.clone()),
            Self::ValueExact(value) => Self::ValueExact(value.clone()),
            Self::ValueRegex(regex) => Self::ValueRegex(regex.clone()),
            Self::Depth(depth) => Self::Depth(*depth),
            Self::Exists => Self::Exists,
            Self::Missing => Self::Missing,
            Self::ValueType(vt) => Self::ValueType(*vt),
            Self::Custom(_) => {
                // Custom predicates cannot be cloned, so we create a default All condition
                // In practice, queries with custom predicates should not be cloned
                Self::All
            }
        }
    }
}

/// Value type conditions for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTypeCondition {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}

impl Query {
    /// Create a new empty query
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            combine_mode: CombineMode::All,
        }
    }

    /// Create a query that selects all keys
    pub fn all() -> Self {
        let mut query = Self::new();
        query.conditions.push(QueryCondition::All);
        query
    }

    /// Create a query for specific key paths
    pub fn keys<I, K>(keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<KeyPath>,
    {
        let mut query = Self::new();
        let key_paths: Vec<KeyPath> = keys.into_iter().map(|k| k.into()).collect();
        query.conditions.push(QueryCondition::KeyPaths(key_paths));
        query
    }

    /// Create a query for a single key path
    pub fn key<K: Into<KeyPath>>(key: K) -> Self {
        Self::keys(vec![key.into()])
    }

    /// Create a query for exact key matches (no hierarchical matching)
    pub fn exact_keys<I, K>(keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<KeyPath>,
    {
        let mut query = Self::new();
        let key_paths: Vec<KeyPath> = keys.into_iter().map(|k| k.into()).collect();
        query
            .conditions
            .push(QueryCondition::ExactKeyPaths(key_paths));
        query
    }

    /// Create a query for a single exact key path (no hierarchical matching)
    pub fn exact_key<K: Into<KeyPath>>(key: K) -> Self {
        Self::exact_keys(vec![key.into()])
    }

    /// Create a query using key regex
    pub fn key_regex(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;
        let mut query = Self::new();
        query.conditions.push(QueryCondition::KeyRegex(regex));
        Ok(query)
    }

    /// Create a query for exact value matches
    pub fn value_exact(value: FrontMatterValue) -> Self {
        let mut query = Self::new();
        query.conditions.push(QueryCondition::ValueExact(value));
        query
    }

    /// Create a query using value regex
    pub fn value_regex(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;
        let mut query = Self::new();
        query.conditions.push(QueryCondition::ValueRegex(regex));
        Ok(query)
    }

    /// Create a query for keys at a specific depth
    pub fn depth(depth: usize) -> Self {
        let mut query = Self::new();
        query.conditions.push(QueryCondition::Depth(depth));
        query
    }

    /// Create a query for existing (non-null) values
    pub fn exists() -> Self {
        let mut query = Self::new();
        query.conditions.push(QueryCondition::Exists);
        query
    }

    /// Create a query for missing or null values
    pub fn missing() -> Self {
        let mut query = Self::new();
        query.conditions.push(QueryCondition::Missing);
        query
    }

    /// Create a query for specific value types
    pub fn value_type(type_condition: ValueTypeCondition) -> Self {
        let mut query = Self::new();
        query
            .conditions
            .push(QueryCondition::ValueType(type_condition));
        query
    }

    /// Add a condition to this query
    pub fn and(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self.combine_mode = CombineMode::All;
        self
    }

    /// Add a key path condition
    pub fn and_key<K: Into<KeyPath>>(mut self, key: K) -> Self {
        self.conditions
            .push(QueryCondition::KeyPaths(vec![key.into()]));
        self
    }

    /// Add an exact key condition (no hierarchical matching)
    pub fn and_exact_key<K: Into<KeyPath>>(mut self, key: K) -> Self {
        self.conditions
            .push(QueryCondition::ExactKeyPaths(vec![key.into()]));
        self
    }

    /// Add a key regex condition
    pub fn and_key_regex(mut self, pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;
        self.conditions.push(QueryCondition::KeyRegex(regex));
        Ok(self)
    }

    /// Add a value condition
    pub fn and_value(mut self, value: FrontMatterValue) -> Self {
        self.conditions.push(QueryCondition::ValueExact(value));
        self
    }

    /// Add a value regex condition
    pub fn and_value_regex(mut self, pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;
        self.conditions.push(QueryCondition::ValueRegex(regex));
        Ok(self)
    }

    /// Add a depth condition
    pub fn and_depth(mut self, depth: usize) -> Self {
        self.conditions.push(QueryCondition::Depth(depth));
        self
    }

    /// Add an exists condition
    pub fn and_exists(mut self) -> Self {
        self.conditions.push(QueryCondition::Exists);
        self
    }

    /// Add a type condition
    pub fn and_type(mut self, type_condition: ValueTypeCondition) -> Self {
        self.conditions
            .push(QueryCondition::ValueType(type_condition));
        self
    }

    /// Add a custom predicate
    pub fn and_custom<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&KeyPath, &FrontMatterValue) -> bool + Send + Sync + 'static,
    {
        self.conditions
            .push(QueryCondition::Custom(Box::new(predicate)));
        self
    }

    /// Change combine mode to OR
    pub fn or(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self.combine_mode = CombineMode::Any;
        self
    }

    /// Set combine mode explicitly
    pub fn combine_with(mut self, mode: CombineMode) -> Self {
        self.combine_mode = mode;
        self
    }

    /// Test if a key-value pair matches this query
    pub fn matches(&self, key_path: &KeyPath, value: &FrontMatterValue) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        let matches: Vec<bool> = self
            .conditions
            .iter()
            .map(|condition| self.matches_condition(condition, key_path, value))
            .collect();

        match self.combine_mode {
            CombineMode::All => matches.iter().all(|&m| m),
            CombineMode::Any => matches.iter().any(|&m| m),
        }
    }

    /// Check if a condition matches
    fn matches_condition(
        &self,
        condition: &QueryCondition,
        key_path: &KeyPath,
        value: &FrontMatterValue,
    ) -> bool {
        match condition {
            QueryCondition::All => true,
            QueryCondition::KeyPaths(paths) => paths
                .iter()
                .any(|path| key_path.starts_with(path) || path.starts_with(key_path)),
            QueryCondition::ExactKeyPaths(paths) => paths.iter().any(|path| key_path == path),
            QueryCondition::KeyRegex(regex) => regex.is_match(&key_path.to_dot_notation()),
            QueryCondition::ValueExact(expected) => value.as_inner() == expected.as_inner(),
            QueryCondition::ValueRegex(regex) => regex.is_match(&value.to_string_representation()),
            QueryCondition::Depth(expected_depth) => key_path.len() == *expected_depth,
            QueryCondition::Exists => !value.is_null(),
            QueryCondition::Missing => value.is_null(),
            QueryCondition::ValueType(type_condition) => {
                self.matches_value_type(value, *type_condition)
            }
            QueryCondition::Custom(predicate) => predicate(key_path, value),
        }
    }

    /// Check if a value matches a type condition
    fn matches_value_type(&self, value: &FrontMatterValue, condition: ValueTypeCondition) -> bool {
        match condition {
            ValueTypeCondition::String => value.is_string(),
            ValueTypeCondition::Number => value.is_number(),
            ValueTypeCondition::Boolean => value.is_bool(),
            ValueTypeCondition::Array => value.is_array(),
            ValueTypeCondition::Object => value.is_object(),
            ValueTypeCondition::Null => value.is_null(),
        }
    }

    /// Get the conditions in this query
    pub fn conditions(&self) -> &[QueryCondition] {
        &self.conditions
    }

    /// Get the combine mode
    pub fn combine_mode(&self) -> CombineMode {
        self.combine_mode
    }

    /// Check if this query is empty (no conditions)
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Check if this query selects all
    pub fn is_select_all(&self) -> bool {
        self.conditions.len() == 1 && matches!(self.conditions[0], QueryCondition::All)
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("conditions", &self.conditions)
            .field("combine_mode", &self.combine_mode)
            .finish()
    }
}

impl Clone for Query {
    fn clone(&self) -> Self {
        Self {
            conditions: self.conditions.clone(),
            combine_mode: self.combine_mode,
        }
    }
}

/// Query result containing matched key-value pairs
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub matches: BTreeMap<KeyPath, FrontMatterValue>,
}

impl QueryResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            matches: BTreeMap::new(),
        }
    }

    /// Create a result from a map
    pub fn from_map(matches: BTreeMap<KeyPath, FrontMatterValue>) -> Self {
        Self { matches }
    }

    /// Add a match to the result
    pub fn add_match(&mut self, key_path: KeyPath, value: FrontMatterValue) {
        self.matches.insert(key_path, value);
    }

    /// Get all matches
    pub fn matches(&self) -> &BTreeMap<KeyPath, FrontMatterValue> {
        &self.matches
    }

    /// Get the number of matches
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Get a value by key path
    pub fn get(&self, key_path: &KeyPath) -> Option<&FrontMatterValue> {
        self.matches.get(key_path)
    }

    /// Convert to a flat YAML value for output
    pub fn to_yaml_value(&self) -> serde_yaml::Value {
        if self.matches.is_empty() {
            return serde_yaml::Value::Null;
        }

        // If there's only one match, return just the value (not wrapped in structure)
        if self.matches.len() == 1 {
            let (_, value) = self.matches.iter().next().unwrap();
            return value.as_inner().clone();
        }

        // Build a nested structure
        let mut root = serde_yaml::Mapping::new();

        for (key_path, value) in &self.matches {
            insert_nested_value(&mut root, key_path.segments(), value.as_inner().clone());
        }

        serde_yaml::Value::Mapping(root)
    }
}

impl Default for QueryResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to insert a nested value into a YAML mapping
fn insert_nested_value(
    root: &mut serde_yaml::Mapping,
    path_segments: &[String],
    value: serde_yaml::Value,
) {
    if path_segments.is_empty() {
        return;
    }

    if path_segments.len() == 1 {
        let key = serde_yaml::Value::String(path_segments[0].clone());
        root.insert(key, value);
        return;
    }

    let key = serde_yaml::Value::String(path_segments[0].clone());

    // Check if the next segment is a numeric index (array access)
    if path_segments.len() >= 2 {
        if let Ok(index) = path_segments[1].parse::<usize>() {
            // We need to create/ensure an array exists
            if !root.contains_key(&key) {
                root.insert(key.clone(), serde_yaml::Value::Sequence(vec![]));
            }

            if let Some(serde_yaml::Value::Sequence(array)) = root.get_mut(&key) {
                // Extend array if necessary
                while array.len() <= index {
                    array.push(serde_yaml::Value::Null);
                }

                if path_segments.len() == 2 {
                    // Set the array element directly
                    array[index] = value;
                } else {
                    // Need to set nested value within array element
                    if !matches!(array[index], serde_yaml::Value::Mapping(_)) {
                        array[index] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    }
                    if let serde_yaml::Value::Mapping(nested_map) = &mut array[index] {
                        insert_nested_value(nested_map, &path_segments[2..], value);
                    }
                }
                return;
            }
        }
    }

    // Handle object path (original logic)
    if !root.contains_key(&key) {
        root.insert(
            key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    // Get the intermediate mapping and recurse
    if let Some(serde_yaml::Value::Mapping(nested_map)) = root.get_mut(&key) {
        insert_nested_value(nested_map, &path_segments[1..], value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_all() {
        let query = Query::all();
        assert!(query.is_select_all());

        let key_path = KeyPath::parse("test").unwrap();
        let value = FrontMatterValue::string("value");
        assert!(query.matches(&key_path, &value));
    }

    #[test]
    fn test_query_key_path() {
        let query = Query::key("title");
        let title_path = KeyPath::parse("title").unwrap();
        let other_path = KeyPath::parse("author").unwrap();
        let value = FrontMatterValue::string("Hello World");

        assert!(query.matches(&title_path, &value));
        assert!(!query.matches(&other_path, &value));
    }

    #[test]
    fn test_query_key_regex() {
        let query = Query::key_regex("^tag").unwrap();
        let tags_path = KeyPath::parse("tags").unwrap();
        let title_path = KeyPath::parse("title").unwrap();
        let value = FrontMatterValue::array(vec![]);

        assert!(query.matches(&tags_path, &value));
        assert!(!query.matches(&title_path, &value));
    }

    #[test]
    fn test_query_value_exact() {
        let expected = FrontMatterValue::string("test");
        let query = Query::value_exact(expected.clone());
        let key_path = KeyPath::parse("key").unwrap();

        assert!(query.matches(&key_path, &expected));
        assert!(!query.matches(&key_path, &FrontMatterValue::string("other")));
    }

    #[test]
    fn test_query_value_regex() {
        let query = Query::value_regex("^Hello").unwrap();
        let key_path = KeyPath::parse("key").unwrap();
        let matching_value = FrontMatterValue::string("Hello World");
        let non_matching_value = FrontMatterValue::string("Goodbye");

        assert!(query.matches(&key_path, &matching_value));
        assert!(!query.matches(&key_path, &non_matching_value));
    }

    #[test]
    fn test_query_depth() {
        let query = Query::depth(2);
        let shallow_path = KeyPath::parse("title").unwrap();
        let deep_path = KeyPath::parse("author.name").unwrap();
        let deeper_path = KeyPath::parse("author.contact.email").unwrap();
        let value = FrontMatterValue::string("test");

        assert!(!query.matches(&shallow_path, &value));
        assert!(query.matches(&deep_path, &value));
        assert!(!query.matches(&deeper_path, &value));
    }

    #[test]
    fn test_query_exists() {
        let query = Query::exists();
        let key_path = KeyPath::parse("key").unwrap();
        let existing_value = FrontMatterValue::string("value");
        let null_value = FrontMatterValue::null();

        assert!(query.matches(&key_path, &existing_value));
        assert!(!query.matches(&key_path, &null_value));
    }

    #[test]
    fn test_query_value_type() {
        let string_query = Query::value_type(ValueTypeCondition::String);
        let number_query = Query::value_type(ValueTypeCondition::Number);

        let key_path = KeyPath::parse("key").unwrap();
        let string_value = FrontMatterValue::string("test");
        let number_value = FrontMatterValue::int(42);

        assert!(string_query.matches(&key_path, &string_value));
        assert!(!string_query.matches(&key_path, &number_value));
        assert!(number_query.matches(&key_path, &number_value));
        assert!(!number_query.matches(&key_path, &string_value));
    }

    #[test]
    fn test_query_and_combination() {
        let query = Query::key("title").and_type(ValueTypeCondition::String);
        let key_path = KeyPath::parse("title").unwrap();
        let other_path = KeyPath::parse("count").unwrap();
        let string_value = FrontMatterValue::string("Hello");
        let number_value = FrontMatterValue::int(42);

        // Both conditions must match
        assert!(query.matches(&key_path, &string_value));
        assert!(!query.matches(&key_path, &number_value)); // Wrong type
        assert!(!query.matches(&other_path, &string_value)); // Wrong key
    }

    #[test]
    fn test_query_or_combination() {
        let query = Query::key("title")
            .or(QueryCondition::KeyPaths(vec![
                KeyPath::parse("author").unwrap()
            ]))
            .combine_with(CombineMode::Any);

        let title_path = KeyPath::parse("title").unwrap();
        let author_path = KeyPath::parse("author").unwrap();
        let other_path = KeyPath::parse("other").unwrap();
        let value = FrontMatterValue::string("test");

        // Either condition can match
        assert!(query.matches(&title_path, &value));
        assert!(query.matches(&author_path, &value));
        assert!(!query.matches(&other_path, &value));
    }

    #[test]
    fn test_query_custom_predicate() {
        let query = Query::new().and_custom(|key_path, value| {
            key_path.segments().len() == 1
                && value.as_string().map(|s| s.len() > 5).unwrap_or(false)
        });

        let key_path = KeyPath::parse("title").unwrap();
        let long_value = FrontMatterValue::string("Hello World");
        let short_value = FrontMatterValue::string("Hi");

        assert!(query.matches(&key_path, &long_value));
        assert!(!query.matches(&key_path, &short_value));
    }

    #[test]
    fn test_query_result() {
        let mut result = QueryResult::new();
        result.add_match(
            KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Hello"),
        );
        result.add_match(KeyPath::parse("count").unwrap(), FrontMatterValue::int(42));

        assert_eq!(result.len(), 2);
        assert!(!result.is_empty());
        assert_eq!(
            result
                .get(&KeyPath::parse("title").unwrap())
                .unwrap()
                .as_string(),
            Some("Hello")
        );
    }

    #[test]
    fn test_query_result_to_yaml() {
        let mut result = QueryResult::new();
        result.add_match(
            KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Hello"),
        );
        result.add_match(
            KeyPath::parse("author.name").unwrap(),
            FrontMatterValue::string("John"),
        );

        let yaml_value = result.to_yaml_value();
        assert!(yaml_value.is_mapping());

        let map = yaml_value.as_mapping().unwrap();
        assert_eq!(
            map.get(&serde_yaml::Value::String("title".to_string()))
                .unwrap()
                .as_str(),
            Some("Hello")
        );
        assert!(map
            .get(&serde_yaml::Value::String("author".to_string()))
            .is_some());
    }

    #[test]
    fn test_query_exact_key_matching() {
        use crate::core::path::KeyPath;
        use crate::core::value::FrontMatterValue;

        // Test hierarchical matching (default behavior)
        let hierarchical_query = Query::key("tags.0");
        let tags_path = KeyPath::parse("tags").unwrap();
        let tags_0_path = KeyPath::parse("tags.0").unwrap();
        let value = FrontMatterValue::string("test");

        // Hierarchical matching should match both parent and child
        assert!(hierarchical_query.matches(&tags_path, &value)); // "tags" matches because "tags.0" starts with "tags"
        assert!(hierarchical_query.matches(&tags_0_path, &value)); // "tags.0" matches exactly

        // Test exact matching (no hierarchical behavior)
        let exact_query = Query::exact_key("tags.0");

        // Exact matching should only match the exact path
        assert!(!exact_query.matches(&tags_path, &value)); // "tags" should not match
        assert!(exact_query.matches(&tags_0_path, &value)); // "tags.0" should match exactly

        // Test with multiple exact keys
        let multi_exact_query = Query::exact_keys(vec!["tags.0", "tags.1"]);
        let tags_1_path = KeyPath::parse("tags.1").unwrap();

        assert!(!multi_exact_query.matches(&tags_path, &value)); // "tags" should not match
        assert!(multi_exact_query.matches(&tags_0_path, &value)); // "tags.0" should match
        assert!(multi_exact_query.matches(&tags_1_path, &value)); // "tags.1" should match

        // Test single exact key queries separately
        let author_name_query = Query::exact_key("author.name");
        let author_email_query = Query::exact_key("author.email");

        let author_path = KeyPath::parse("author").unwrap();
        let author_name_path = KeyPath::parse("author.name").unwrap();
        let author_email_path = KeyPath::parse("author.email").unwrap();

        // Test author.name query
        assert!(!author_name_query.matches(&author_path, &value)); // "author" should not match
        assert!(author_name_query.matches(&author_name_path, &value)); // "author.name" should match
        assert!(!author_name_query.matches(&author_email_path, &value)); // "author.email" should not match

        // Test author.email query
        assert!(!author_email_query.matches(&author_path, &value)); // "author" should not match
        assert!(!author_email_query.matches(&author_name_path, &value)); // "author.name" should not match
        assert!(author_email_query.matches(&author_email_path, &value)); // "author.email" should match
    }
}
