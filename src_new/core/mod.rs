//! Core library types and functionality for front matter manipulation
//!
//! This module contains the main types and traits for working with front matter:
//! - Document: The main document type representing a markdown file with front matter
//! - FrontMatterValue: Type-safe wrapper for YAML values
//! - KeyPath: Parsed key paths for nested access
//! - Query: Composable query system for filtering and selecting data

pub mod document;
pub mod path;
pub mod query;
pub mod value;

pub use document::Document;
pub use path::KeyPath;
pub use query::{CombineMode, Query, QueryResult, ValueTypeCondition};
pub use value::{FrontMatterValue, ValueType};
