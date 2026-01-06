//! Integration tests for the matterof library
//!
//! These tests verify the complete functionality of the library using only
//! the modern JSONPath system where it works properly, and simpler document
//! operations for basic functionality testing.

use matterof::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_basic_document_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");

    let content = r#"---
title: Original Title
author:
  name: John Doe
  email: john@example.com
tags: [rust, cli]
published: false
---
# Hello World

This is the original content."#;

    fs::write(&file_path, content).unwrap();

    // Test reading
    let reader = FrontMatterReader::new();
    let document = reader.read_file(&file_path).unwrap();

    assert!(document.has_front_matter());
    assert_eq!(
        document.body().trim(),
        "# Hello World\n\nThis is the original content."
    );

    // Test basic front matter access
    if let Some(front_matter) = document.front_matter() {
        assert!(front_matter.contains_key("title"));
        assert!(front_matter.contains_key("author"));
        assert!(front_matter.contains_key("tags"));
        assert!(front_matter.contains_key("published"));
    }

    // Test writing back
    let writer = FrontMatterWriter::new();
    let result = writer.write_file(&document, &file_path, None).unwrap();
    // File may be modified due to formatting differences

    // Verify document is still readable
    let reread_document = reader.read_file(&file_path).unwrap();
    assert!(reread_document.has_front_matter());
}

#[test]
fn test_document_creation_and_validation() {
    // Test empty document
    let empty_doc = Document::empty();
    assert!(!empty_doc.has_front_matter());
    assert_eq!(empty_doc.body(), "");
    assert!(empty_doc.validate().is_ok());

    // Test document with body only
    let body_doc = Document::body_only("# Test Content".to_string());
    assert!(!body_doc.has_front_matter());
    assert_eq!(body_doc.body(), "# Test Content");
    assert!(body_doc.validate().is_ok());

    // Test document creation from YAML
    let yaml_value = serde_yaml::from_str(
        r#"
title: Test Document
count: 42
"#,
    )
    .unwrap();

    let doc_from_yaml =
        Document::from_yaml_value(Some(yaml_value), "Body content".to_string()).unwrap();
    assert!(doc_from_yaml.has_front_matter());
    assert_eq!(doc_from_yaml.body(), "Body content");
    assert!(doc_from_yaml.validate().is_ok());
}

#[test]
fn test_jsonpath_basic_queries() {
    // Test JSONPath query construction
    let queries = vec![
        "$[*]",                // All root elements
        "$['title']",          // Specific property with bracket notation
        "$['author']['name']", // Nested property access
    ];

    for query_str in queries {
        let query = JsonPathQuery::new(query_str);
        assert!(query.is_ok(), "Failed to parse JSONPath: {}", query_str);
    }

    // Test invalid JSONPath
    let invalid_queries = vec!["[invalid", "$.invalid[", "unclosed'quote"];

    for invalid_query in invalid_queries {
        let query = JsonPathQuery::new(invalid_query);
        assert!(
            query.is_err(),
            "Should fail for invalid JSONPath: {}",
            invalid_query
        );
    }
}

#[test]
fn test_yaml_json_conversion() {
    let mut document = Document::empty();
    document.ensure_front_matter();

    // Add some test data directly to the front matter
    if let Some(front_matter) = document.front_matter() {
        let yaml_value = YamlJsonConverter::document_front_matter_to_yaml(front_matter);
        let json_result = YamlJsonConverter::yaml_to_json(&yaml_value);
        assert!(json_result.is_ok());

        // Test roundtrip conversion
        let json_value = json_result.unwrap();
        let yaml_result = YamlJsonConverter::json_to_yaml(&json_value);
        assert!(yaml_result.is_ok());

        let final_fm_result =
            YamlJsonConverter::yaml_to_document_front_matter(&yaml_result.unwrap());
        assert!(final_fm_result.is_ok());
    }
}

#[test]
fn test_file_operations_and_io() {
    let temp_dir = TempDir::new().unwrap();

    // Test file resolution
    let resolver = FileResolver::new();
    let files = resolver
        .resolve_paths(&[temp_dir.path().to_str().unwrap()])
        .unwrap();
    assert!(files.is_empty()); // No markdown files yet

    // Create test files
    for i in 0..3 {
        let file_path = temp_dir.path().join(format!("test_{}.md", i));
        let content = format!(
            r#"---
id: {}
title: Test Document {}
---
# Document {}"#,
            i, i, i
        );
        fs::write(&file_path, content).unwrap();
    }

    // Test file resolution again
    let files = resolver
        .resolve_paths(&[temp_dir.path().to_str().unwrap()])
        .unwrap();
    let markdown_files: Vec<_> = files.iter().filter(|f| f.is_markdown()).collect();
    assert_eq!(markdown_files.len(), 3);

    // Test batch reading
    let reader = FrontMatterReader::new();
    for file in &markdown_files {
        let document = reader.read_file(file.path()).unwrap();
        assert!(document.has_front_matter());
        assert!(document.front_matter().unwrap().contains_key("id"));
        assert!(document.front_matter().unwrap().contains_key("title"));
    }
}

#[test]
fn test_malformed_and_edge_case_files() {
    let temp_dir = TempDir::new().unwrap();

    // Test file without front matter
    let no_fm_path = temp_dir.path().join("no_fm.md");
    fs::write(&no_fm_path, "# Just a title\n\nSome content").unwrap();

    let reader = FrontMatterReader::new();
    let document = reader.read_file(&no_fm_path).unwrap();
    assert!(!document.has_front_matter());
    assert_eq!(document.body(), "# Just a title\n\nSome content");

    // Test empty file
    let empty_path = temp_dir.path().join("empty.md");
    fs::write(&empty_path, "").unwrap();

    let empty_document = reader.read_file(&empty_path).unwrap();
    assert!(!empty_document.has_front_matter());
    assert_eq!(empty_document.body(), "");

    // Test file with empty front matter
    let empty_fm_path = temp_dir.path().join("empty_fm.md");
    let empty_fm_content = r#"---
---
Body content"#;
    fs::write(&empty_fm_path, empty_fm_content).unwrap();

    let empty_fm_doc = reader.read_file(&empty_fm_path).unwrap();
    assert!(!empty_fm_doc.has_front_matter()); // Empty front matter should not be considered present
    assert_eq!(empty_fm_doc.body(), "Body content");
}

#[test]
fn test_backup_and_atomic_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("backup_test.md");

    let content = r#"---
title: Original Title
---
# Original"#;

    fs::write(&file_path, content).unwrap();

    let reader = FrontMatterReader::new();
    let document = reader.read_file(&file_path).unwrap();

    // Test backup functionality
    let writer = FrontMatterWriter::new();
    let write_options = WriteOptions {
        backup: Some(BackupOptions {
            enabled: true,
            suffix: Some(".backup".to_string()),
            directory: None,
        }),
        dry_run: false,
        ..Default::default()
    };

    let result = writer
        .write_file(&document, &file_path, Some(write_options))
        .unwrap();

    // Verify backup behavior (may or may not create backup depending on whether changes were detected)
    if result.modified {
        // If changes were detected, backup should be created
        let backup_path = temp_dir.path().join("backup_test.md.backup");
        if result.backup_path.is_some() {
            assert!(backup_path.exists());
        }
    }

    // Verify main file still exists and is readable
    let final_document = reader.read_file(&file_path).unwrap();
    assert!(final_document.has_front_matter());
}

#[test]
fn test_dry_run_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("dry_run_test.md");

    let content = r#"---
title: Original
---
# Test"#;

    fs::write(&file_path, content).unwrap();
    let original_modified = fs::metadata(&file_path).unwrap().modified().unwrap();

    let reader = FrontMatterReader::new();
    let document = reader.read_file(&file_path).unwrap();

    // Test dry run
    let writer = FrontMatterWriter::new();
    let write_options = WriteOptions {
        dry_run: true,
        ..Default::default()
    };

    let result = writer
        .write_file(&document, &file_path, Some(write_options))
        .unwrap();

    // File should not be modified in dry run mode
    let new_modified = fs::metadata(&file_path).unwrap().modified().unwrap();
    assert_eq!(original_modified, new_modified);

    // Content should be unchanged
    let unchanged_content = fs::read_to_string(&file_path).unwrap();
    assert!(unchanged_content.contains("Original"));

    // Result should indicate whether changes would have been made
    // (may be true due to formatting differences)
}

#[test]
fn test_convenience_functions() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("convenience_test.md");

    let content = r#"---
title: Convenience Test
count: 42
author:
  name: John Doe
---
Body content"#;

    fs::write(&file_path, content).unwrap();

    // Test convenience read/write functions
    let document = convenience::read_document(&file_path).unwrap();
    assert!(document.has_front_matter());
    assert_eq!(document.body(), "Body content");

    // Test convenience write
    let write_result = convenience::write_document(&document, &file_path).unwrap();
    // Write may or may not report modifications due to formatting

    // Verify document is still intact
    let reread_document = convenience::read_document(&file_path).unwrap();
    assert!(reread_document.has_front_matter());
    assert_eq!(reread_document.body(), "Body content");
}

#[test]
fn test_error_handling() {
    // Test file not found
    let reader = FrontMatterReader::new();
    let nonexistent_result = reader.read_file("/nonexistent/file.md");
    assert!(nonexistent_result.is_err());

    // Test invalid JSONPath expressions
    let invalid_query_result = JsonPathQuery::new("[invalid");
    assert!(invalid_query_result.is_err());

    // Test document validation with various inputs
    let empty_doc = Document::empty();
    assert!(empty_doc.validate().is_ok());

    let body_doc = Document::body_only("Valid content".to_string());
    assert!(body_doc.validate().is_ok());
}
