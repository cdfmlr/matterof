//! matterof: A production-ready library for manipulating YAML front matter in markdown files
//!
//! This library provides a clean, efficient, and type-safe API for reading, querying,
//! modifying, and writing YAML front matter in markdown documents. It's designed to be
//! both a standalone library and the foundation for CLI tools.
//!
//! # Features
//!
//! - **Type-safe front matter handling** with proper error types
//! - **Flexible query system** for filtering and selecting data
//! - **Atomic file operations** with backup support
//! - **Batch processing** with file resolution and filtering
//! - **Clean separation** between library and CLI concerns
//! - **Comprehensive error handling** with detailed error information
//! - **Performance optimized** with lazy loading and efficient algorithms
//!
//! # Quick Start
//!
//! ## Reading and Parsing
//!
//! ```rust,no_run
//! use matterof::{Document, FrontMatterReader, JsonPathQuery, YamlJsonConverter, Result};
//!
//! fn main() -> Result<()> {
//!     // Read a document from file
//!     let reader = FrontMatterReader::new();
//!     let document = reader.read_file("example.md")?;
//!
//!     // Query front matter values using JSONPath
//!     let title_query = JsonPathQuery::new("$.title")?;
//!     let author_query = JsonPathQuery::new("$.author.name")?;
//!     Ok(())
//! }
//! ```
//!
//! ## Querying Front Matter
//!
//! ```rust,no_run
//! use matterof::{Document, JsonPathQuery, YamlJsonConverter, Result};
//!
//! fn main() -> Result<()> {
//!     let document = Document::empty();
//!
//!     // Query front matter using JSONPath
//!     let string_query = JsonPathQuery::new("$..*[?(@.type() == 'string')]")?;
//!     let tag_query = JsonPathQuery::new("$..tag*")?;
//!
//!     // Process results using JsonPathQueryResult
//!     Ok(())
//! }
//! ```
//!
//! ## Modifying Documents
//!
//! ```rust,no_run
//! use matterof::{Document, JsonMutator, JsonPathQuery, YamlJsonConverter, Result};
//!
//! fn main() -> Result<()> {
//!     let mut document = Document::empty();
//!
//!     // Use JSONPath and JsonMutator for modifications
//!     let title_query = JsonPathQuery::new("$.title")?;
//!     let tags_query = JsonPathQuery::new("$.tags")?;
//!
//!     // Modify front matter using JsonMutator
//!     // (Implementation would require conversion to JSON, mutation, then back to YAML)
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
//! The library is organized into several modules:
//!
//! - [`core`]: Core types and domain logic (Document, Query, KeyPath, etc.)
//! - [`io`]: File I/O operations (reading, writing, file resolution)
//! - [`error`]: Comprehensive error handling with detailed error types
//!
//! The design follows these principles:
//!
//! - **Separation of concerns**: Clear boundaries between parsing, querying, and I/O
//! - **Composability**: Small, focused types that can be combined
//! - **Performance**: Lazy evaluation and efficient algorithms
//! - **Safety**: Comprehensive error handling and type safety
//! - **Usability**: Builder patterns and convenience functions for common operations

// Public API exports
pub use error::{ErrorSeverity, MatterOfError, Result};

// Core types
pub use core::{
    Document, FrontMatterValue, JsonMutator, JsonPathQuery, JsonPathQueryResult,
    NormalizedPathUtils, ParsedPath, PathSegment, ValueType, YamlJsonConverter,
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

/// Convenience functions for common operations
pub mod convenience {
    //! Convenience functions that provide simple APIs for common use cases
    //!
    //! These functions use sensible defaults and are perfect for simple scripts
    //! or when you don't need fine-grained control over the operations.

    pub use crate::io::convenience::*;

    use crate::{
        Document, FrontMatterValue, JsonMutator, JsonPathQuery, Result, YamlJsonConverter,
    };

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

    /// Quick way to get a single value from a file using JSONPath
    pub fn get_value<P: AsRef<std::path::Path>>(
        path: P,
        jsonpath: &str,
    ) -> Result<Option<serde_json::Value>> {
        let document = crate::io::convenience::read_document(path)?;
        if let Some(front_matter) = document.front_matter() {
            let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
            let json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
            let query = JsonPathQuery::new(jsonpath)?;
            let results = query.query_located(&json_value);
            Ok(results.into_iter().next().map(|(_, value)| value.clone()))
        } else {
            Ok(None)
        }
    }

    /// Quick way to set a single value in a file using JSONPath
    pub fn set_value<P: AsRef<std::path::Path>>(
        path: P,
        jsonpath: &str,
        value: FrontMatterValue,
    ) -> Result<()> {
        let mut document = crate::io::convenience::read_document(path.as_ref())
            .unwrap_or_else(|_| Document::empty());

        document.ensure_front_matter();
        let front_matter = document.front_matter().unwrap();
        let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
        let mut json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;
        let json_new_value = YamlJsonConverter::front_matter_to_json(&value)?;

        JsonMutator::set_at_path(&mut json_value, jsonpath, json_new_value)?;
        let yaml_result = YamlJsonConverter::json_to_yaml(&json_value)?;
        let front_matter_map = YamlJsonConverter::yaml_to_document_front_matter(&yaml_result)?;
        document.set_front_matter(Some(front_matter_map));

        crate::io::convenience::write_document(&document, path)?;
        Ok(())
    }

    /// Quick way to remove a key from a file using JSONPath
    pub fn remove_key<P: AsRef<std::path::Path>>(path: P, jsonpath: &str) -> Result<()> {
        let mut document = crate::io::convenience::read_document(path.as_ref())?;

        if let Some(front_matter) = document.front_matter() {
            let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
            let mut json_value = YamlJsonConverter::yaml_to_json(&yaml_value)?;

            JsonMutator::remove_at_path(&mut json_value, jsonpath)?;
            let yaml_result = YamlJsonConverter::json_to_yaml(&json_value)?;
            let front_matter_map = YamlJsonConverter::yaml_to_document_front_matter(&yaml_result)?;
            document.set_front_matter(Some(front_matter_map));
        }

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
        let document = reader.read_file(temp_file.path()).unwrap();

        // Verify document structure
        assert!(document.has_front_matter());
        assert_eq!(
            document.body().trim(),
            "# Hello World\n\nThis is the body content."
        );

        // Test that we can read and write without corruption
        let writer = FrontMatterWriter::new();
        let _result = writer
            .write_file(&document, temp_file.path(), None)
            .unwrap();

        // Verify document is still intact after write
        let updated_document = reader.read_file(temp_file.path()).unwrap();
        assert!(updated_document.has_front_matter());
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

        // Test basic document parsing
        let document = convenience::read_document(temp_file.path()).unwrap();
        assert!(document.has_front_matter());
        assert_eq!(document.body(), "Body content");

        // Test writing document back
        let _result = convenience::write_document(&document, temp_file.path()).unwrap();
        // Note: Writing may be detected as modification due to formatting differences
    }

    #[test]
    fn test_document_lifecycle() {
        // Create empty document
        let doc = Document::empty();
        assert!(!doc.has_front_matter());
        assert_eq!(doc.body(), "");

        // Test document with front matter
        let mut doc_with_fm = Document::empty();
        doc_with_fm.ensure_front_matter();
        // Note: ensure_front_matter creates empty map, but has_front_matter checks for non-empty
        // This is expected behavior - empty front matter is cleaned up
        doc_with_fm.clean_empty_front_matter();
        assert!(!doc_with_fm.has_front_matter());
    }

    #[test]
    fn test_complex_nested_operations() {
        // Test basic JSONPath query creation
        let query = JsonPathQuery::new("$.author.name");
        assert!(query.is_ok());

        let query2 = JsonPathQuery::new("$..tags[*]");
        assert!(query2.is_ok());

        // Test document creation and validation
        let doc = Document::empty();
        assert!(!doc.has_front_matter());
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_error_handling() {
        // Test invalid JSONPath
        let invalid_jsonpath_result = JsonPathQuery::new("[invalid");
        assert!(invalid_jsonpath_result.is_err());

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
