//! File resolution for handling multiple files and directories
//!
//! This module provides utilities for resolving file paths, handling wildcards,
//! filtering files, and managing batch operations across multiple files.

use crate::error::{MatterOfError, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration for file resolution
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Follow symbolic links
    pub follow_links: bool,
    /// Maximum recursion depth for directory traversal
    pub max_depth: Option<usize>,
    /// Include hidden files (starting with .)
    pub include_hidden: bool,
    /// File extensions to include (if empty, includes all markdown files)
    pub include_extensions: Vec<String>,
    /// File extensions to exclude
    pub exclude_extensions: Vec<String>,
    /// Patterns to exclude (glob-style)
    pub exclude_patterns: Vec<String>,
    /// Only include files that exist
    pub only_existing: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: None,
            include_hidden: false,
            include_extensions: vec!["md".to_string(), "markdown".to_string()],
            exclude_extensions: Vec::new(),
            exclude_patterns: Vec::new(),
            only_existing: true,
        }
    }
}

/// File resolver for handling multiple files and directories
pub struct FileResolver {
    config: ResolverConfig,
}

/// Result of file resolution
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// The resolved file path
    pub path: PathBuf,
    /// Whether this file is a markdown file
    pub is_markdown: bool,
    /// Whether this file exists
    pub exists: bool,
}

impl FileResolver {
    /// Create a new resolver with default configuration
    pub fn new() -> Self {
        Self {
            config: ResolverConfig::default(),
        }
    }

    /// Create a new resolver with custom configuration
    pub fn with_config(config: ResolverConfig) -> Self {
        Self { config }
    }

    /// Resolve multiple paths to a list of files
    pub fn resolve_paths<P>(&self, paths: &[P]) -> Result<Vec<ResolvedFile>>
    where
        P: AsRef<Path>,
    {
        let mut resolved_files = Vec::new();
        let mut seen_paths = HashSet::new();

        for path in paths {
            let path = path.as_ref();
            let files = self.resolve_single_path(path)?;

            for file in files {
                // Avoid duplicates
                if seen_paths.insert(file.path.clone()) {
                    resolved_files.push(file);
                }
            }
        }

        // Filter based on configuration
        resolved_files = self.filter_files(resolved_files)?;

        // Sort for consistent ordering
        resolved_files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(resolved_files)
    }

    /// Resolve a single path (file or directory)
    fn resolve_single_path(&self, path: &Path) -> Result<Vec<ResolvedFile>> {
        let mut resolved_files = Vec::new();

        if path.is_file() {
            resolved_files.push(ResolvedFile {
                path: path.to_path_buf(),
                is_markdown: self.is_markdown_file(path),
                exists: true,
            });
        } else if path.is_dir() {
            let files = self.traverse_directory(path)?;
            resolved_files.extend(files);
        } else if !self.config.only_existing {
            // Path doesn't exist, but we might want to include it anyway
            resolved_files.push(ResolvedFile {
                path: path.to_path_buf(),
                is_markdown: self.is_markdown_file(path),
                exists: false,
            });
        } else {
            return Err(MatterOfError::file_not_found(path));
        }

        Ok(resolved_files)
    }

    /// Traverse a directory and collect files
    fn traverse_directory(&self, dir_path: &Path) -> Result<Vec<ResolvedFile>> {
        let mut resolved_files = Vec::new();

        let walker = WalkDir::new(dir_path)
            .follow_links(self.config.follow_links)
            .max_depth(self.config.max_depth.unwrap_or(usize::MAX))
            .into_iter();

        for entry in walker {
            let entry = entry.map_err(|e| {
                MatterOfError::path_resolution(format!("Error traversing directory: {}", e))
            })?;

            let path = entry.path();

            // Skip directories
            if !entry.file_type().is_file() {
                continue;
            }

            // Skip hidden files if not included
            if !self.config.include_hidden && self.is_hidden_file(path) {
                continue;
            }

            resolved_files.push(ResolvedFile {
                path: path.to_path_buf(),
                is_markdown: self.is_markdown_file(path),
                exists: true,
            });
        }

        Ok(resolved_files)
    }

    /// Filter files based on configuration
    fn filter_files(&self, files: Vec<ResolvedFile>) -> Result<Vec<ResolvedFile>> {
        let mut filtered = Vec::new();

        for file in files {
            // Skip non-existent files if only_existing is true
            if self.config.only_existing && !file.exists {
                continue;
            }

            // Check include extensions
            if !self.config.include_extensions.is_empty() {
                let ext = file
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                if !self.config.include_extensions.contains(&ext) {
                    continue;
                }
            }

            // Check exclude extensions
            if !self.config.exclude_extensions.is_empty() {
                let ext = file
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                if self.config.exclude_extensions.contains(&ext) {
                    continue;
                }
            }

            // Check exclude patterns (simple glob-like matching)
            if self.should_exclude_by_pattern(&file.path)? {
                continue;
            }

            filtered.push(file);
        }

        Ok(filtered)
    }

    /// Check if a file should be excluded by pattern matching
    fn should_exclude_by_pattern(&self, path: &Path) -> Result<bool> {
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(path, pattern)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Simple pattern matching (supports * and ? wildcards)
    fn matches_pattern(&self, path: &Path, pattern: &str) -> Result<bool> {
        let path_str = path.to_string_lossy();

        // Convert simple glob pattern to regex
        let regex_pattern = pattern
            .replace(".", r"\.")
            .replace("*", ".*")
            .replace("?", ".");

        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))?;
        Ok(regex.is_match(&path_str))
    }

    /// Check if a file is a markdown file based on extension
    fn is_markdown_file(&self, path: &Path) -> bool {
        match path.extension() {
            Some(ext) => {
                let ext_str = ext.to_string_lossy().to_lowercase();
                matches!(
                    ext_str.as_str(),
                    "md" | "markdown" | "mdown" | "mkd" | "mkdn"
                )
            }
            None => false,
        }
    }

    /// Check if a file is hidden (starts with .)
    fn is_hidden_file(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with('.'))
            .unwrap_or(false)
    }

    /// Get only markdown files from resolved files
    pub fn markdown_files(files: &[ResolvedFile]) -> Vec<&ResolvedFile> {
        files.iter().filter(|f| f.is_markdown).collect()
    }

    /// Get only existing files from resolved files
    pub fn existing_files(files: &[ResolvedFile]) -> Vec<&ResolvedFile> {
        files.iter().filter(|f| f.exists).collect()
    }

    /// Get the resolver configuration
    pub fn config(&self) -> &ResolverConfig {
        &self.config
    }
}

impl Default for FileResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolvedFile {
    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if this is a markdown file
    pub fn is_markdown(&self) -> bool {
        self.is_markdown
    }

    /// Check if this file exists
    pub fn exists(&self) -> bool {
        self.exists
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|s| s.to_str())
    }

    /// Get the filename
    pub fn filename(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    /// Get the parent directory
    pub fn parent(&self) -> Option<&Path> {
        self.path.parent()
    }
}

/// Convenience functions for common operations
pub mod convenience {
    use super::*;

    /// Resolve paths to markdown files with default settings
    pub fn resolve_markdown_files<P>(paths: &[P]) -> Result<Vec<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let resolver = FileResolver::new();
        let resolved = resolver.resolve_paths(paths)?;
        Ok(FileResolver::markdown_files(&resolved)
            .into_iter()
            .map(|f| f.path.clone())
            .collect())
    }

    /// Resolve paths to all files with default settings
    pub fn resolve_all_files<P>(paths: &[P]) -> Result<Vec<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let config = ResolverConfig {
            include_extensions: Vec::new(), // Include all files
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(paths)?;
        Ok(resolved.into_iter().map(|f| f.path).collect())
    }

    /// Check if a single path is a markdown file
    pub fn is_markdown_file<P: AsRef<Path>>(path: P) -> bool {
        FileResolver::new().is_markdown_file(path.as_ref())
    }

    /// Resolve a single directory to markdown files
    pub fn resolve_directory<P: AsRef<Path>>(dir_path: P) -> Result<Vec<PathBuf>> {
        let resolver = FileResolver::new();
        let resolved = resolver.resolve_paths(&[dir_path])?;
        Ok(FileResolver::markdown_files(&resolved)
            .into_iter()
            .map(|f| f.path.clone())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files(dir: &Path) -> Result<()> {
        fs::write(dir.join("test1.md"), "# Test 1")?;
        fs::write(dir.join("test2.markdown"), "# Test 2")?;
        fs::write(dir.join("readme.txt"), "Not markdown")?;
        fs::write(dir.join(".hidden.md"), "# Hidden")?;

        let subdir = dir.join("subdir");
        fs::create_dir(&subdir)?;
        fs::write(subdir.join("nested.md"), "# Nested")?;

        Ok(())
    }

    #[test]
    fn test_resolve_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "# Test").unwrap();

        let resolver = FileResolver::new();
        let resolved = resolver.resolve_paths(&[&file_path]).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].path, file_path);
        assert!(resolved[0].is_markdown);
        assert!(resolved[0].exists);
    }

    #[test]
    fn test_resolve_directory() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let resolver = FileResolver::new();
        let resolved = resolver.resolve_paths(&[temp_dir.path()]).unwrap();

        // Should find test1.md, test2.markdown, and nested.md (but not .hidden.md or readme.txt)
        let markdown_files: Vec<_> = resolved.iter().filter(|f| f.is_markdown).collect();
        assert_eq!(markdown_files.len(), 3);

        let filenames: Vec<_> = markdown_files
            .iter()
            .map(|f| f.filename().unwrap())
            .collect();
        assert!(filenames.contains(&"test1.md"));
        assert!(filenames.contains(&"test2.markdown"));
        assert!(filenames.contains(&"nested.md"));
    }

    #[test]
    fn test_include_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let config = ResolverConfig {
            include_hidden: true,
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(&[temp_dir.path()]).unwrap();

        let markdown_files: Vec<_> = resolved.iter().filter(|f| f.is_markdown).collect();
        assert_eq!(markdown_files.len(), 4); // Including .hidden.md

        let filenames: Vec<_> = markdown_files
            .iter()
            .map(|f| f.filename().unwrap())
            .collect();
        assert!(filenames.contains(&".hidden.md"));
    }

    #[test]
    fn test_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let config = ResolverConfig {
            max_depth: Some(1),
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(&[temp_dir.path()]).unwrap();

        // Should only find files in root directory (depth 0), not in subdir
        let markdown_files: Vec<_> = resolved.iter().filter(|f| f.is_markdown).collect();
        assert_eq!(markdown_files.len(), 2); // test1.md and test2.markdown only

        let filenames: Vec<_> = markdown_files
            .iter()
            .map(|f| f.filename().unwrap())
            .collect();
        assert!(filenames.contains(&"test1.md"));
        assert!(filenames.contains(&"test2.markdown"));
        assert!(!filenames.contains(&"nested.md"));
    }

    #[test]
    fn test_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let config = ResolverConfig {
            exclude_patterns: vec!["*test1*".to_string()],
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(&[temp_dir.path()]).unwrap();

        let markdown_files: Vec<_> = resolved.iter().filter(|f| f.is_markdown).collect();
        let filenames: Vec<_> = markdown_files
            .iter()
            .map(|f| f.filename().unwrap())
            .collect();

        assert!(!filenames.contains(&"test1.md"));
        assert!(filenames.contains(&"test2.markdown"));
    }

    #[test]
    fn test_include_extensions() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let config = ResolverConfig {
            include_extensions: vec!["md".to_string()],
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(&[temp_dir.path()]).unwrap();

        // Should only include .md files, not .markdown
        let filenames: Vec<_> = resolved.iter().map(|f| f.filename().unwrap()).collect();

        assert!(filenames.contains(&"test1.md"));
        assert!(!filenames.contains(&"test2.markdown")); // Excluded because it's .markdown, not .md
        assert!(filenames.contains(&"nested.md"));
    }

    #[test]
    fn test_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.md");

        let resolver = FileResolver::new();
        let result = resolver.resolve_paths(&[&nonexistent]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MatterOfError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_include_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.md");

        let config = ResolverConfig {
            only_existing: false,
            ..Default::default()
        };
        let resolver = FileResolver::with_config(config);
        let resolved = resolver.resolve_paths(&[&nonexistent]).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].path, nonexistent);
        assert!(resolved[0].is_markdown);
        assert!(!resolved[0].exists);
    }

    #[test]
    fn test_convenience_functions() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        // Test resolve_markdown_files
        let markdown_files = convenience::resolve_markdown_files(&[temp_dir.path()]).unwrap();
        assert_eq!(markdown_files.len(), 3); // test1.md, test2.markdown, nested.md

        // Test is_markdown_file
        assert!(convenience::is_markdown_file("test.md"));
        assert!(convenience::is_markdown_file("test.markdown"));
        assert!(!convenience::is_markdown_file("test.txt"));

        // Test resolve_directory
        let dir_files = convenience::resolve_directory(temp_dir.path()).unwrap();
        assert_eq!(dir_files.len(), 3);
    }

    #[test]
    fn test_duplicate_removal() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        fs::write(&file_path, "# Test").unwrap();

        let resolver = FileResolver::new();
        // Pass the same path twice
        let resolved = resolver.resolve_paths(&[&file_path, &file_path]).unwrap();

        // Should only appear once
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].path, file_path);
    }
}
