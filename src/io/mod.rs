//! IO operations for reading and writing front matter files
//!
//! This module provides the complete IO stack for working with front matter files:
//! - Reader: Efficient file reading and front matter parsing
//! - Writer: Safe file writing with atomic operations and backup support
//! - Resolver: File path resolution and filtering for batch operations

pub mod reader;
pub mod resolver;
pub mod writer;

pub use reader::{FrontMatterReader, ReaderConfig};
pub use resolver::{FileResolver, ResolvedFile, ResolverConfig};
pub use writer::{
    BackupOptions, FrontMatterWriter, LineEndings, OutputOptions, WriteOptions, WriteResult,
    WriterConfig,
};

/// Re-export convenience functions for easy access
pub mod convenience {
    pub use super::reader::convenience::*;
    pub use super::resolver::convenience::*;
    pub use super::writer::convenience::*;
}
