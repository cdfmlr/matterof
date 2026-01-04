//! Core document model for front matter manipulation
//!
//! This module provides the main Document type that represents a markdown file
//! with front matter, offering clean APIs for reading, querying, and modifying
//! the front matter while preserving the document body.

use crate::core::{
    path::KeyPath,
    query::{Query, QueryResult},
    value::FrontMatterValue,
};
use crate::error::{MatterOfError, Result};
use std::collections::BTreeMap;

/// Represents a markdown document with front matter and body
#[derive(Debug, Clone)]
pub struct Document {
    front_matter: Option<BTreeMap<String, FrontMatterValue>>,
    body: String,
    original_content: Option<String>,
}

impl Document {
    /// Create a new document with optional front matter and body
    pub fn new(front_matter: Option<BTreeMap<String, FrontMatterValue>>, body: String) -> Self {
        Self {
            front_matter,
            body,
            original_content: None,
        }
    }

    /// Create a new empty document
    pub fn empty() -> Self {
        Self::new(None, String::new())
    }

    /// Create a document with only body content (no front matter)
    pub fn body_only(body: String) -> Self {
        Self::new(None, body)
    }

    /// Create a document from a YAML value and body
    pub fn from_yaml_value(yaml_value: Option<serde_yaml::Value>, body: String) -> Result<Self> {
        let front_matter = match yaml_value {
            Some(serde_yaml::Value::Mapping(map)) => {
                let mut fm = BTreeMap::new();
                for (k, v) in map {
                    if let Some(key_str) = k.as_str() {
                        fm.insert(key_str.to_string(), FrontMatterValue::new(v));
                    }
                }
                Some(fm)
            }
            Some(serde_yaml::Value::Null) | None => None,
            Some(other) => {
                return Err(MatterOfError::invalid_front_matter(
                    "<unknown>",
                    format!("Expected mapping or null, found {:?}", other),
                ));
            }
        };

        Ok(Self::new(front_matter, body))
    }

    /// Set the original content for change tracking
    pub fn with_original_content(mut self, content: String) -> Self {
        self.original_content = Some(content);
        self
    }

    /// Get the front matter as a reference
    pub fn front_matter(&self) -> Option<&BTreeMap<String, FrontMatterValue>> {
        self.front_matter.as_ref()
    }

    /// Get the body content
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Get the original content if available
    pub fn original_content(&self) -> Option<&str> {
        self.original_content.as_deref()
    }

    /// Check if the document has front matter
    pub fn has_front_matter(&self) -> bool {
        self.front_matter
            .as_ref()
            .map(|fm| !fm.is_empty())
            .unwrap_or(false)
    }

    /// Check if the document has been modified
    pub fn is_modified(&self) -> bool {
        // This is a simple check - in a real implementation you might want
        // more sophisticated change tracking
        self.original_content.is_some()
    }

    /// Initialize front matter if it doesn't exist
    pub fn ensure_front_matter(&mut self) {
        if self.front_matter.is_none() {
            self.front_matter = Some(BTreeMap::new());
        }
    }

    /// Remove front matter if it's empty
    pub fn clean_empty_front_matter(&mut self) {
        if let Some(ref fm) = self.front_matter {
            if fm.is_empty() {
                self.front_matter = None;
            }
        }
    }

    /// Get a value by key path
    pub fn get(&self, key_path: &KeyPath) -> Option<FrontMatterValue> {
        let fm = self.front_matter.as_ref()?;
        self.get_nested_value(fm, key_path.segments())
    }

    /// Set a value at the given key path
    pub fn set(&mut self, key_path: &KeyPath, value: FrontMatterValue) -> Result<()> {
        self.ensure_front_matter();
        let segments = key_path.segments().to_vec();
        let fm = self.front_matter.as_mut().unwrap();
        Self::set_nested_value_static(fm, &segments, value)?;
        Ok(())
    }

    /// Remove a key path
    pub fn remove(&mut self, key_path: &KeyPath) -> Result<Option<FrontMatterValue>> {
        let segments = key_path.segments().to_vec();
        let fm = match self.front_matter.as_mut() {
            Some(fm) => fm,
            None => return Ok(None),
        };

        let removed = Self::remove_nested_value_static(fm, &segments)?;
        self.clean_empty_front_matter();
        Ok(removed)
    }

    /// Add a value to an array at the given key path
    pub fn add_to_array(
        &mut self,
        key_path: &KeyPath,
        value: FrontMatterValue,
        index: Option<usize>,
    ) -> Result<()> {
        self.ensure_front_matter();

        // Get or create the array
        let current_value = self.get(key_path);
        let mut array_values = match current_value {
            Some(val) if val.is_array() => val.as_array().unwrap(),
            Some(val) if val.is_null() => Vec::new(),
            Some(val) => vec![val], // Convert single value to array
            None => Vec::new(),
        };

        // Insert at the specified index or append
        match index {
            Some(idx) if idx <= array_values.len() => {
                array_values.insert(idx, value);
            }
            _ => {
                array_values.push(value);
            }
        }

        // Set the updated array
        self.set(key_path, FrontMatterValue::array(array_values))?;
        Ok(())
    }

    /// Remove a value from an array at the given key path
    pub fn remove_from_array(
        &mut self,
        key_path: &KeyPath,
        value: &FrontMatterValue,
    ) -> Result<bool> {
        let current_value = match self.get(key_path) {
            Some(val) if val.is_array() => val,
            _ => return Ok(false),
        };

        let mut array_values = current_value.as_array().unwrap();
        let original_len = array_values.len();

        // Remove matching values
        array_values.retain(|v| v.as_inner() != value.as_inner());

        if array_values.len() != original_len {
            self.set(key_path, FrontMatterValue::array(array_values))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Query the front matter using a query
    pub fn query(&self, query: &Query) -> QueryResult {
        let mut result = QueryResult::new();

        if let Some(ref fm) = self.front_matter {
            self.query_recursive(fm, &KeyPath::new(), query, &mut result);
        }

        result
    }

    /// Update the body content
    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    /// Merge another document's front matter into this one
    pub fn merge_front_matter(&mut self, other: &Document) -> Result<()> {
        if let Some(ref other_fm) = other.front_matter {
            self.ensure_front_matter();
            let fm = self.front_matter.as_mut().unwrap();

            for (key, value) in other_fm {
                if let Some(existing) = fm.get_mut(key) {
                    existing.merge(value.clone())?;
                } else {
                    fm.insert(key.clone(), value.clone());
                }
            }
        }
        Ok(())
    }

    /// Convert to YAML value representation
    pub fn to_yaml_value(&self) -> serde_yaml::Value {
        match &self.front_matter {
            Some(fm) => {
                let mut map = serde_yaml::Mapping::new();
                for (key, value) in fm {
                    map.insert(
                        serde_yaml::Value::String(key.clone()),
                        value.as_inner().clone(),
                    );
                }
                serde_yaml::Value::Mapping(map)
            }
            None => serde_yaml::Value::Null,
        }
    }

    /// Validate the front matter structure
    pub fn validate(&self) -> Result<()> {
        if let Some(ref _fm) = self.front_matter {
            // Check for any invalid YAML structures
            let yaml_value = self.to_yaml_value();

            // Try to serialize and deserialize to catch any issues
            let serialized = serde_yaml::to_string(&yaml_value)?;
            let _: serde_yaml::Value = serde_yaml::from_str(&serialized)?;
        }
        Ok(())
    }

    /// Get a flattened view of all key-value pairs
    pub fn flatten(&self) -> BTreeMap<KeyPath, FrontMatterValue> {
        let mut flattened = BTreeMap::new();

        if let Some(ref fm) = self.front_matter {
            self.flatten_recursive(fm, &KeyPath::new(), &mut flattened);
        }

        flattened
    }

    // Private helper methods

    fn get_nested_value(
        &self,
        container: &BTreeMap<String, FrontMatterValue>,
        path: &[String],
    ) -> Option<FrontMatterValue> {
        if path.is_empty() {
            return None;
        }

        let value = container.get(&path[0])?;

        if path.len() == 1 {
            Some(value.clone())
        } else if let Some(nested_map) = value.as_object() {
            self.get_nested_value(&nested_map, &path[1..])
        } else {
            None
        }
    }

    fn set_nested_value_static(
        container: &mut BTreeMap<String, FrontMatterValue>,
        path: &[String],
        value: FrontMatterValue,
    ) -> Result<()> {
        if path.is_empty() {
            return Err(MatterOfError::invalid_key_path(
                "<empty>",
                "Cannot set value at empty path",
            ));
        }

        if path.len() == 1 {
            container.insert(path[0].clone(), value);
            return Ok(());
        }

        // Ensure intermediate path exists
        let key = &path[0];
        if !container.contains_key(key) {
            container.insert(key.clone(), FrontMatterValue::object(BTreeMap::new()));
        }

        // Get the nested container
        let nested_value = container.get_mut(key).unwrap();
        if !nested_value.is_object() {
            // Convert to object if it's not already
            *nested_value = FrontMatterValue::object(BTreeMap::new());
        }

        let mut nested_map = nested_value.as_object().unwrap();
        Self::set_nested_value_static(&mut nested_map, &path[1..], value)?;

        // Update the nested value
        container.insert(key.clone(), FrontMatterValue::object(nested_map));
        Ok(())
    }

    fn remove_nested_value_static(
        container: &mut BTreeMap<String, FrontMatterValue>,
        path: &[String],
    ) -> Result<Option<FrontMatterValue>> {
        if path.is_empty() {
            return Ok(None);
        }

        if path.len() == 1 {
            return Ok(container.remove(&path[0]));
        }

        let key = &path[0];
        let nested_value = match container.get_mut(key) {
            Some(value) if value.is_object() => value,
            _ => return Ok(None),
        };

        let mut nested_map = nested_value.as_object().unwrap();
        let result = Self::remove_nested_value_static(&mut nested_map, &path[1..])?;

        // Update the nested container or remove it if empty
        if nested_map.is_empty() {
            container.remove(key);
        } else {
            container.insert(key.clone(), FrontMatterValue::object(nested_map));
        }

        Ok(result)
    }

    fn query_recursive(
        &self,
        container: &BTreeMap<String, FrontMatterValue>,
        current_path: &KeyPath,
        query: &Query,
        result: &mut QueryResult,
    ) {
        for (key, value) in container {
            let key_path = current_path.child(key);

            // Test this key-value pair
            if query.matches(&key_path, value) {
                result.add_match(key_path.clone(), value.clone());
            }

            // Recurse into nested objects
            if let Some(nested_map) = value.as_object() {
                self.query_recursive(&nested_map, &key_path, query, result);
            }
        }
    }

    fn flatten_recursive(
        &self,
        container: &BTreeMap<String, FrontMatterValue>,
        current_path: &KeyPath,
        result: &mut BTreeMap<KeyPath, FrontMatterValue>,
    ) {
        for (key, value) in container {
            let key_path = current_path.child(key);
            result.insert(key_path.clone(), value.clone());

            if let Some(nested_map) = value.as_object() {
                self.flatten_recursive(&nested_map, &key_path, result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::empty();
        assert!(!doc.has_front_matter());
        assert_eq!(doc.body(), "");

        let doc = Document::body_only("# Hello World".to_string());
        assert!(!doc.has_front_matter());
        assert_eq!(doc.body(), "# Hello World");
    }

    #[test]
    fn test_front_matter_operations() {
        let mut doc = Document::empty();
        let key_path = KeyPath::parse("title").unwrap();
        let value = FrontMatterValue::string("Hello World");

        // Set value
        doc.set(&key_path, value.clone()).unwrap();
        assert!(doc.has_front_matter());

        // Get value
        let retrieved = doc.get(&key_path).unwrap();
        assert_eq!(retrieved.as_string(), Some("Hello World"));

        // Remove value
        let removed = doc.remove(&key_path).unwrap();
        assert!(removed.is_some());
        assert!(!doc.has_front_matter()); // Should be cleaned up
    }

    #[test]
    fn test_nested_operations() {
        let mut doc = Document::empty();
        let nested_path = KeyPath::parse("author.name").unwrap();
        let value = FrontMatterValue::string("John Doe");

        doc.set(&nested_path, value).unwrap();

        let retrieved = doc.get(&nested_path).unwrap();
        assert_eq!(retrieved.as_string(), Some("John Doe"));

        // Check that intermediate structure was created
        let author = doc.get(&KeyPath::parse("author").unwrap()).unwrap();
        assert!(author.is_object());
    }

    #[test]
    fn test_array_operations() {
        let mut doc = Document::empty();
        let tags_path = KeyPath::parse("tags").unwrap();

        // Add to non-existent array
        doc.add_to_array(&tags_path, FrontMatterValue::string("rust"), None)
            .unwrap();
        doc.add_to_array(&tags_path, FrontMatterValue::string("cli"), None)
            .unwrap();

        let tags = doc.get(&tags_path).unwrap();
        let tag_values = tags.as_array().unwrap();
        assert_eq!(tag_values.len(), 2);
        assert_eq!(tag_values[0].as_string(), Some("rust"));
        assert_eq!(tag_values[1].as_string(), Some("cli"));

        // Insert at specific index
        doc.add_to_array(&tags_path, FrontMatterValue::string("tool"), Some(1))
            .unwrap();

        let updated_tags = doc.get(&tags_path).unwrap();
        let updated_values = updated_tags.as_array().unwrap();
        assert_eq!(updated_values.len(), 3);
        assert_eq!(updated_values[1].as_string(), Some("tool"));
    }

    #[test]
    fn test_query_operations() {
        let mut doc = Document::empty();

        // Set up test data
        doc.set(
            &KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Hello"),
        )
        .unwrap();
        doc.set(&KeyPath::parse("count").unwrap(), FrontMatterValue::int(42))
            .unwrap();
        doc.set(
            &KeyPath::parse("author.name").unwrap(),
            FrontMatterValue::string("John"),
        )
        .unwrap();

        // Query for all string values
        let query = Query::new().and_type(crate::core::query::ValueTypeCondition::String);
        let result = doc.query(&query);

        assert_eq!(result.len(), 2); // title and author.name
        assert!(result.get(&KeyPath::parse("title").unwrap()).is_some());
        assert!(result
            .get(&KeyPath::parse("author.name").unwrap())
            .is_some());

        // Query for specific key
        let title_query = Query::key("title");
        let title_result = doc.query(&title_query);
        assert_eq!(title_result.len(), 1);
        assert_eq!(
            title_result
                .get(&KeyPath::parse("title").unwrap())
                .unwrap()
                .as_string(),
            Some("Hello")
        );
    }

    #[test]
    fn test_document_merge() {
        let mut doc1 = Document::empty();
        doc1.set(
            &KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Doc1"),
        )
        .unwrap();
        doc1.set(&KeyPath::parse("count").unwrap(), FrontMatterValue::int(1))
            .unwrap();

        let mut doc2 = Document::empty();
        doc2.set(
            &KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Doc2"),
        )
        .unwrap();
        doc2.set(
            &KeyPath::parse("author").unwrap(),
            FrontMatterValue::string("John"),
        )
        .unwrap();

        doc1.merge_front_matter(&doc2).unwrap();

        // doc2's title should overwrite doc1's title
        assert_eq!(
            doc1.get(&KeyPath::parse("title").unwrap())
                .unwrap()
                .as_string(),
            Some("Doc2")
        );

        // Other values should be preserved/added
        assert_eq!(
            doc1.get(&KeyPath::parse("count").unwrap())
                .unwrap()
                .as_int(),
            Some(1)
        );
        assert_eq!(
            doc1.get(&KeyPath::parse("author").unwrap())
                .unwrap()
                .as_string(),
            Some("John")
        );
    }

    #[test]
    fn test_document_validation() {
        let doc = Document::empty();
        assert!(doc.validate().is_ok());

        let mut doc_with_data = Document::empty();
        doc_with_data
            .set(
                &KeyPath::parse("title").unwrap(),
                FrontMatterValue::string("Test"),
            )
            .unwrap();
        assert!(doc_with_data.validate().is_ok());
    }

    #[test]
    fn test_flatten() {
        let mut doc = Document::empty();
        doc.set(
            &KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Hello"),
        )
        .unwrap();
        doc.set(
            &KeyPath::parse("author.name").unwrap(),
            FrontMatterValue::string("John"),
        )
        .unwrap();
        doc.set(
            &KeyPath::parse("author.email").unwrap(),
            FrontMatterValue::string("john@example.com"),
        )
        .unwrap();

        let flattened = doc.flatten();
        assert_eq!(flattened.len(), 4); // title, author, author.name, author.email

        assert!(flattened.contains_key(&KeyPath::parse("title").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("author").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("author.name").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("author.email").unwrap()));
    }
}
