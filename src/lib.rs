//! matterof: A library and CLI tool for manipulating YAML front matter in markdown files
//!
//! This library provides a clean, efficient, and type-safe API for reading, querying,
//! modifying, and writing YAML front matter in markdown documents. It is designed to be
//! both a standalone library and the foundation for the `matterof` CLI tool.
//!
//! # Features
//!
//! - **Type-safe front matter handling** with proper error types
//! - **JSONPath query support** for powerful filtering and selection
//! - **Atomic file operations** with backup support
//! - **Batch processing** with file resolution and filtering
//! - **Clean separation** between library and CLI concerns
//! - **Comprehensive error handling** with detailed error information
//!
//! # Quick Start
//!
//! ## Reading and Parsing
//!
//! ```rust,no_run
//! use matterof::{FrontMatterReader, KeyPath, Result};
//!
//! fn main() -> Result<()> {
//!     // Read a document from file
//!     let reader = FrontMatterReader::new();
//!     let document = reader.read_file("example.md")?;
//!
//!     // Access front matter values by key path
//!     let title = document.get(&KeyPath::parse("title")?);
//!     let author_name = document.get(&KeyPath::parse("author.name")?);
//!     println!("Title: {:?}", title);
//!     println!("Author: {:?}", author_name);
//!     Ok(())
//! }
//! ```
//!
//! ## Querying Front Matter
//!
//! ```rust,no_run
//! use matterof::{Document, FrontMatterReader, JsonPathQuery, Result};
//!
//! fn main() -> Result<()> {
//!     let reader = FrontMatterReader::new();
//!     let document = reader.read_file("example.md")?;
//!
//!     // Use JSONPath to query front matter (auto-prepends "$." if needed)
//!     if let Some(fm) = document.front_matter() {
//!         use matterof::YamlJsonConverter;
//!         let yaml = YamlJsonConverter::document_front_matter_to_yaml(fm);
//!         let json = YamlJsonConverter::yaml_to_json(&yaml)?;
//!         let q = JsonPathQuery::new("tags[*]")?;
//!         let results = q.query(&json);
//!         for tag in results {
//!             println!("tag: {}", tag);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Modifying Documents
//!
//! ```rust,no_run
//! use matterof::{Document, FrontMatterValue, KeyPath, Result};
//!
//! fn main() -> Result<()> {
//!     let mut document = Document::empty();
//!
//!     // Set values with automatic type conversion
//!     document.set(
//!         &KeyPath::parse("title")?,
//!         FrontMatterValue::string("My Post")
//!     )?;
//!
//!     // Add to arrays
//!     document.add_to_array(
//!         &KeyPath::parse("tags")?,
//!         FrontMatterValue::string("rust"),
//!         None
//!     )?;
//!     Ok(())
//! }
//! ```
//!
//! ## Writing Files
//!
//! ```rust,no_run
//! use matterof::{Document, FrontMatterWriter, WriteOptions, BackupOptions, Result};
//!
//! fn main() -> Result<()> {
//!     let document = Document::empty();
//!     let writer = FrontMatterWriter::new();
//!     let options = WriteOptions {
//!         backup: Some(BackupOptions {
//!             enabled: true,
//!             suffix: Some(".backup".to_string()),
//!             directory: None,
//!         }),
//!         dry_run: false,
//!         ..Default::default()
//!     };
//!
//!     let result = writer.write_file(&document, "output.md", Some(options))?;
//!     Ok(())
//! }
//! ```
//!
//! ## Batch Operations
//!
//! ```rust,no_run
//! use matterof::{FileResolver, FrontMatterReader, Result};
//!
//! fn main() -> Result<()> {
//!     // Resolve multiple files and directories
//!     let resolver = FileResolver::new();
//!     let files = resolver.resolve_paths(&["docs/", "README.md"])?;
//!     let reader = FrontMatterReader::new();
//!
//!     for file in files {
//!         if file.is_markdown() && file.exists() {
//!             let doc = reader.read_file(file.path())?;
//!             // Process document...
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The library is organised into several modules:
//!
//! - [`core`]: Core types and domain logic (Document, KeyPath, Query, JSONPath support, etc.)
//! - [`io`]: File I/O operations (reading, writing, file resolution)
//! - [`error`]: Comprehensive error handling with detailed error types
//!
//! The design follows these principles:
//!
//! - **Separation of concerns**: Clear boundaries between parsing, querying, and I/O
//! - **Composability**: Small, focused types that can be combined
//! - **Safety**: Comprehensive error handling and type safety
//! - **Usability**: Builder patterns and convenience functions for common operations

// Public API exports
pub use error::{ErrorSeverity, MatterOfError, Result};

// Core types
pub use core::{
    CombineMode, Document, FrontMatterValue, JsonPathQuery, JsonPathQueryResult, KeyPath,
    NormalizedPathUtils, Query, QueryResult, ValueType, ValueTypeCondition, YamlJsonConverter,
};

// IO types
pub use io::{
    BackupOptions, FileResolver, FrontMatterReader, FrontMatterWriter, LineEndings, OutputOptions,
    ReaderConfig, ResolvedFile, ResolverConfig, WriteOptions, WriteResult, WriterConfig,
};

// Internal modules
pub mod core;
pub mod error;
pub mod io;

// CLI components are available only in the binary, not as part of the library API

/// Convenience functions for common single-step operations.
///
/// These functions use sensible defaults and are perfect for simple scripts
/// or when you don't need fine-grained control over the operations.
pub mod convenience {
    pub use crate::io::convenience::*;

    use crate::{Document, FrontMatterValue, KeyPath, MatterOfError, Result};

    /// Parse front matter from a string
    pub fn parse_document(content: &str) -> Result<Document> {
        crate::io::convenience::parse_document(content)
    }

    /// Read a document from a file
    pub fn read_document<P: AsRef<std::path::Path>>(path: P) -> Result<Document> {
        crate::io::convenience::read_document(path)
    }

    /// Write a document to a file
    pub fn write_document<P: AsRef<std::path::Path>>(
        document: &Document,
        path: P,
    ) -> Result<crate::WriteResult> {
        crate::io::convenience::write_document(document, path)
    }

    /// Quick way to get a single value from a file
    pub fn get_value<P: AsRef<std::path::Path>>(
        path: P,
        key: &str,
    ) -> Result<Option<FrontMatterValue>> {
        let document = crate::io::convenience::read_document(path)?;
        let key_path = KeyPath::parse(key)?;
        Ok(document.get(&key_path))
    }

    /// Quick way to set a single value in a file
    pub fn set_value<P: AsRef<std::path::Path>>(
        path: P,
        key: &str,
        value: FrontMatterValue,
    ) -> Result<()> {
        let mut document = match crate::io::convenience::read_document(path.as_ref()) {
            Ok(doc) => doc,
            Err(MatterOfError::FileNotFound { .. }) => Document::empty(),
            Err(e) => return Err(e),
        };
        let key_path = KeyPath::parse(key)?;
        document.set(&key_path, value)?;
        crate::io::convenience::write_document(&document, path)?;
        Ok(())
    }

    /// Quick way to remove a key from a file
    pub fn remove_key<P: AsRef<std::path::Path>>(path: P, key: &str) -> Result<()> {
        let mut document = crate::io::convenience::read_document(path.as_ref())?;
        let key_path = KeyPath::parse(key)?;
        document.remove(&key_path)?;
        crate::io::convenience::write_document(&document, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_end_to_end_workflow() {
        // Create a test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"---
title: Test Document
author: John Doe
tags: [rust, test]
---
# Hello World

This is the body content."#;
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Read the document
        let reader = FrontMatterReader::new();
        let mut document = reader.read_file(temp_file.path()).unwrap();

        // Query for data
        let title_query = Query::key("title");
        let title_results = document.query(&title_query);
        assert_eq!(title_results.len(), 1);

        // Modify the document
        document
            .set(
                &KeyPath::parse("title").unwrap(),
                FrontMatterValue::string("Modified Title"),
            )
            .unwrap();

        document
            .add_to_array(
                &KeyPath::parse("tags").unwrap(),
                FrontMatterValue::string("modified"),
                None,
            )
            .unwrap();

        // Write back
        let writer = FrontMatterWriter::new();
        let result = writer
            .write_file(&document, temp_file.path(), None)
            .unwrap();
        assert!(result.modified);

        // Verify changes
        let updated_document = reader.read_file(temp_file.path()).unwrap();
        let updated_title = updated_document
            .get(&KeyPath::parse("title").unwrap())
            .unwrap();
        assert_eq!(updated_title.as_string(), Some("Modified Title"));

        let updated_tags = updated_document
            .get(&KeyPath::parse("tags").unwrap())
            .unwrap();
        let tag_array = updated_tags.as_array().unwrap();
        assert_eq!(tag_array.len(), 3);
        assert_eq!(tag_array[2].as_string(), Some("modified"));
    }

    #[test]
    fn test_convenience_functions() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"---
title: Convenience Test
count: 42
---
Body content"#;
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test get_value
        let title_value = convenience::get_value(temp_file.path(), "title")
            .unwrap()
            .unwrap();
        assert_eq!(title_value.as_string(), Some("Convenience Test"));

        // Test set_value
        convenience::set_value(
            temp_file.path(),
            "new_field",
            FrontMatterValue::string("new_value"),
        )
        .unwrap();

        // Verify the change
        let new_value = convenience::get_value(temp_file.path(), "new_field")
            .unwrap()
            .unwrap();
        assert_eq!(new_value.as_string(), Some("new_value"));

        // Test remove_key
        convenience::remove_key(temp_file.path(), "count").unwrap();

        // Verify removal
        let removed_value = convenience::get_value(temp_file.path(), "count").unwrap();
        assert!(removed_value.is_none());
    }

    #[test]
    fn test_document_lifecycle() {
        // Create empty document
        let mut doc = Document::empty();
        assert!(!doc.has_front_matter());

        // Add some data
        doc.set(
            &KeyPath::parse("title").unwrap(),
            FrontMatterValue::string("Test"),
        )
        .unwrap();
        assert!(doc.has_front_matter());

        // Query the data
        let all_query = Query::all();
        let results = doc.query(&all_query);
        assert_eq!(results.len(), 1);

        // Remove the data
        doc.remove(&KeyPath::parse("title").unwrap()).unwrap();
        assert!(!doc.has_front_matter());
    }

    #[test]
    fn test_complex_nested_operations() {
        let mut doc = Document::empty();

        // Create nested structure
        doc.set(
            &KeyPath::parse("author.name").unwrap(),
            FrontMatterValue::string("John Doe"),
        )
        .unwrap();
        doc.set(
            &KeyPath::parse("author.email").unwrap(),
            FrontMatterValue::string("john@example.com"),
        )
        .unwrap();
        doc.set(
            &KeyPath::parse("meta.version").unwrap(),
            FrontMatterValue::int(1),
        )
        .unwrap();

        // Query nested data - the hierarchical matching will return both "author" and "author.name"
        // because Query::key uses prefix matching in both directions
        let author_query = Query::key("author.name");
        let author_results = doc.query(&author_query);
        // Expect 2: "author" (parent) and "author.name" (exact match)
        assert_eq!(author_results.len(), 2);

        let author_name = author_results
            .get(&KeyPath::parse("author.name").unwrap())
            .unwrap();
        assert_eq!(author_name.as_string(), Some("John Doe"));

        // Query with regex for exact matching
        let email_query = Query::key_regex(r"^author\.email$").unwrap();
        let email_results = doc.query(&email_query);
        assert_eq!(email_results.len(), 1);

        // Test flattening
        let flattened = doc.flatten();
        assert!(flattened.contains_key(&KeyPath::parse("author").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("author.name").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("author.email").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("meta").unwrap()));
        assert!(flattened.contains_key(&KeyPath::parse("meta.version").unwrap()));
    }

    #[test]
    fn test_error_handling() {
        // Test invalid key path
        let invalid_path_result = KeyPath::parse("unclosed[\"quote");
        assert!(invalid_path_result.is_err());

        // Test invalid regex
        let invalid_regex_result = Query::key_regex("[invalid");
        assert!(invalid_regex_result.is_err());

        // Test file not found
        let reader = FrontMatterReader::new();
        let nonexistent_result = reader.read_file("/nonexistent/file.md");
        assert!(nonexistent_result.is_err());
        assert!(matches!(
            nonexistent_result.unwrap_err(),
            MatterOfError::FileNotFound { .. }
        ));
    }
}
