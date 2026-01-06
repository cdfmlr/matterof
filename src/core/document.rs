//! Core document model for front matter manipulation
//!
//! This module provides the main Document type that represents a markdown file
//! with front matter, offering clean APIs for reading, querying, and modifying
//! the front matter while preserving the document body.

use crate::core::value::FrontMatterValue;
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

    /// Update the body content
    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    /// Set the front matter
    pub fn set_front_matter(&mut self, front_matter: Option<BTreeMap<String, FrontMatterValue>>) {
        self.front_matter = front_matter;
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
}

impl Default for Document {
    fn default() -> Self {
        Self::empty()
    }
}
