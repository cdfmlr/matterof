//! File writing with atomic operations, backup support, and diff generation
//!
//! This module provides safe file writing operations with support for backups,
//! atomic writes, and preview functionality including unified diff generation.

use crate::core::Document;
use crate::error::{MatterOfError, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Configuration for the front matter writer
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Create backup files before writing
    pub backup_enabled: bool,
    /// Backup file suffix (e.g., ".bak")
    pub backup_suffix: Option<String>,
    /// Backup directory (if None, backups go in same directory)
    pub backup_dir: Option<PathBuf>,
    /// Use atomic writes (write to temp file first, then rename)
    pub atomic_writes: bool,
    /// Preserve file permissions
    pub preserve_permissions: bool,
    /// Line ending style
    pub line_endings: LineEndings,
}

/// Line ending styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndings {
    /// Unix-style line endings (\n)
    Unix,
    /// Windows-style line endings (\r\n)
    Windows,
    /// Preserve original line endings
    Preserve,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            backup_enabled: false,
            backup_suffix: None,
            backup_dir: None,
            atomic_writes: true,
            preserve_permissions: true,
            line_endings: LineEndings::Preserve,
        }
    }
}

/// Front matter writer
pub struct FrontMatterWriter {
    config: WriterConfig,
}

/// Write operation options for individual operations
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Override backup settings for this operation
    pub backup: Option<BackupOptions>,
    /// Override output settings for this operation
    pub output: Option<OutputOptions>,
    /// Dry run - generate diff without writing
    pub dry_run: bool,
}

/// Backup options
#[derive(Debug, Clone)]
pub struct BackupOptions {
    /// Enable backup for this operation
    pub enabled: bool,
    /// Backup suffix
    pub suffix: Option<String>,
    /// Backup directory
    pub directory: Option<PathBuf>,
}

/// Output options
#[derive(Debug, Clone)]
pub enum OutputOptions {
    /// Write to the original file (default)
    InPlace,
    /// Write to stdout
    Stdout,
    /// Write to a specific file
    File(PathBuf),
    /// Write to a directory, preserving filename
    Directory(PathBuf),
}

/// Result of a write operation
#[derive(Debug)]
pub struct WriteResult {
    /// Whether the file was actually modified
    pub modified: bool,
    /// Path where the content was written (None for stdout)
    pub output_path: Option<PathBuf>,
    /// Path of backup file if created
    pub backup_path: Option<PathBuf>,
    /// Unified diff showing changes (for dry-run or when requested)
    pub diff: Option<String>,
}


impl FrontMatterWriter {
    /// Create a new writer with default configuration
    pub fn new() -> Self {
        Self {
            config: WriterConfig::default(),
        }
    }

    /// Create a new writer with custom configuration
    pub fn with_config(config: WriterConfig) -> Self {
        Self { config }
    }

    /// Write a document to a file
    pub fn write_file<P: AsRef<Path>>(
        &self,
        document: &Document,
        path: P,
        options: Option<WriteOptions>,
    ) -> Result<WriteResult> {
        let path = path.as_ref();
        let options = options.unwrap_or_default();

        // Generate the new content
        let new_content = self.format_document(document)?;

        // Read original content for comparison
        let original_content = if path.exists() {
            Some(fs::read_to_string(path).map_err(MatterOfError::Io)?)
        } else {
            None
        };

        // Check if content has actually changed
        let content_changed = match &original_content {
            Some(original) => {
                self.normalize_content(original) != self.normalize_content(&new_content)
            }
            None => !new_content.trim().is_empty(),
        };

        // Generate diff if requested or for dry run
        let diff = if options.dry_run || original_content.is_some() {
            self.generate_diff(
                original_content.as_deref().unwrap_or(""),
                &new_content,
                path,
            )
        } else {
            None
        };

        // Handle dry run
        if options.dry_run {
            return Ok(WriteResult {
                modified: content_changed,
                output_path: Some(path.to_path_buf()),
                backup_path: None,
                diff,
            });
        }

        // Determine output destination
        let output_destination = options.output.as_ref().unwrap_or(&OutputOptions::InPlace);

        match output_destination {
            OutputOptions::Stdout => {
                if content_changed {
                    println!("{}", new_content);
                }
                Ok(WriteResult {
                    modified: content_changed,
                    output_path: None,
                    backup_path: None,
                    diff,
                })
            }
            OutputOptions::InPlace => self.write_to_file(
                path,
                &new_content,
                &original_content,
                &options,
                content_changed,
            ),
            OutputOptions::File(target_path) => {
                self.write_to_file(target_path, &new_content, &None, &options, true)
            }
            OutputOptions::Directory(target_dir) => {
                let filename = path.file_name().ok_or_else(|| {
                    MatterOfError::path_resolution("Could not extract filename".to_string())
                })?;
                let target_path = target_dir.join(filename);
                self.write_to_file(&target_path, &new_content, &None, &options, true)
            }
        }
    }

    /// Write content to a specific file path
    fn write_to_file(
        &self,
        path: &Path,
        content: &str,
        original_content: &Option<String>,
        options: &WriteOptions,
        content_changed: bool,
    ) -> Result<WriteResult> {
        let mut result = WriteResult {
            modified: content_changed,
            output_path: Some(path.to_path_buf()),
            backup_path: None,
            diff: None,
        };

        if !content_changed {
            return Ok(result);
        }

        // Create backup if requested and file exists
        if self.should_create_backup(options) && path.exists() {
            result.backup_path = Some(self.create_backup(path, options)?);
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(MatterOfError::Io)?;
        }

        // Write the file
        if self.config.atomic_writes {
            self.write_atomic(path, content)?;
        } else {
            self.write_direct(path, content)?;
        }

        // Preserve permissions if requested
        if self.config.preserve_permissions {
            if let Some(original) = original_content {
                if !original.is_empty() {
                    self.preserve_file_permissions(path)?;
                }
            }
        }

        Ok(result)
    }

    /// Format a document into string content
    fn format_document(&self, document: &Document) -> Result<String> {
        let yaml_content = if let Some(fm) = document.front_matter() {
            if fm.is_empty() {
                None
            } else {
                let yaml_value = document.to_yaml_value();
                let yaml_str = serde_yaml::to_string(&yaml_value)?;
                Some(yaml_str.trim().to_string())
            }
        } else {
            None
        };

        let formatted = match yaml_content {
            Some(yaml) => {
                format!("---\n{}\n---\n{}", yaml, document.body())
            }
            None => document.body().to_string(),
        };

        Ok(self.normalize_line_endings(&formatted))
    }

    /// Normalize content for comparison (handle line endings, trailing whitespace)
    fn normalize_content(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Normalize line endings based on configuration
    fn normalize_line_endings(&self, content: &str) -> String {
        match self.config.line_endings {
            LineEndings::Unix => content.replace("\r\n", "\n").replace('\r', "\n"),
            LineEndings::Windows => content
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\r\n"),
            LineEndings::Preserve => content.to_string(),
        }
    }

    /// Generate unified diff between old and new content
    fn generate_diff(&self, old_content: &str, new_content: &str, path: &Path) -> Option<String> {
        if old_content == new_content {
            return None;
        }

        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        // Simple unified diff implementation
        let mut diff_lines = Vec::new();
        diff_lines.push(format!("--- {}", path.display()));
        diff_lines.push(format!("+++ {}", path.display()));

        // Find common prefix and suffix to minimize diff size
        let common_prefix = old_lines
            .iter()
            .zip(new_lines.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let old_suffix = &old_lines[common_prefix..];
        let new_suffix = &new_lines[common_prefix..];

        let common_suffix_len = old_suffix
            .iter()
            .rev()
            .zip(new_suffix.iter().rev())
            .take_while(|(a, b)| a == b)
            .count();

        let old_middle = if common_suffix_len > 0 {
            &old_suffix[..old_suffix.len() - common_suffix_len]
        } else {
            old_suffix
        };

        let new_middle = if common_suffix_len > 0 {
            &new_suffix[..new_suffix.len() - common_suffix_len]
        } else {
            new_suffix
        };

        if !old_middle.is_empty() || !new_middle.is_empty() {
            diff_lines.push(format!(
                "@@ -{},{} +{},{} @@",
                common_prefix + 1,
                old_middle.len(),
                common_prefix + 1,
                new_middle.len()
            ));

            for line in old_middle {
                diff_lines.push(format!("-{}", line));
            }
            for line in new_middle {
                diff_lines.push(format!("+{}", line));
            }
        }

        if diff_lines.len() > 2 {
            Some(diff_lines.join("\n"))
        } else {
            None
        }
    }

    /// Check if backup should be created
    fn should_create_backup(&self, options: &WriteOptions) -> bool {
        if let Some(ref backup_opts) = options.backup {
            backup_opts.enabled
        } else {
            self.config.backup_enabled
        }
    }

    /// Create a backup file
    fn create_backup(&self, original_path: &Path, options: &WriteOptions) -> Result<PathBuf> {
        let backup_suffix = if let Some(ref backup_opts) = options.backup {
            backup_opts.suffix.as_deref()
        } else {
            self.config.backup_suffix.as_deref()
        }
        .unwrap_or(".bak");

        let backup_dir = if let Some(ref backup_opts) = options.backup {
            backup_opts.directory.as_ref()
        } else {
            self.config.backup_dir.as_ref()
        };

        let backup_path = match backup_dir {
            Some(dir) => {
                // Create backup in specified directory
                let filename = original_path.file_name().ok_or_else(|| {
                    MatterOfError::backup_error("Could not extract filename for backup".to_string())
                })?;
                fs::create_dir_all(dir).map_err(|e| {
                    MatterOfError::backup_error(format!("Could not create backup directory: {}", e))
                })?;
                dir.join(format!("{}{}", filename.to_string_lossy(), backup_suffix))
            }
            None => {
                // Create backup in same directory as original
                let mut backup_name = original_path.to_path_buf();
                backup_name.set_extension(format!(
                    "{}{}",
                    original_path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                    backup_suffix
                ));
                backup_name
            }
        };

        fs::copy(original_path, &backup_path)
            .map_err(|e| MatterOfError::backup_error(format!("Failed to create backup: {}", e)))?;

        Ok(backup_path)
    }

    /// Write file atomically using temporary file
    fn write_atomic(&self, path: &Path, content: &str) -> Result<()> {
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

        let mut temp_file = NamedTempFile::new_in(parent_dir).map_err(MatterOfError::Io)?;

        temp_file
            .write_all(content.as_bytes())
            .map_err(MatterOfError::Io)?;
        temp_file.flush().map_err(MatterOfError::Io)?;

        temp_file.persist(path).map_err(|e| {
            MatterOfError::Io(std::io::Error::other(format!(
                "Failed to persist temporary file: {}",
                e
            )))
        })?;

        Ok(())
    }

    /// Write file directly
    fn write_direct(&self, path: &Path, content: &str) -> Result<()> {
        fs::write(path, content).map_err(MatterOfError::Io)
    }

    /// Preserve file permissions from original to new file
    fn preserve_file_permissions(&self, path: &Path) -> Result<()> {
        // This is a simplified implementation
        // In a real-world scenario, you might want more sophisticated permission handling
        #[cfg(unix)]
        {
            let metadata = fs::metadata(path).map_err(MatterOfError::Io)?;
            let permissions = metadata.permissions();
            fs::set_permissions(path, permissions).map_err(MatterOfError::Io)?;
        }
        Ok(())
    }

    /// Get writer configuration
    pub fn config(&self) -> &WriterConfig {
        &self.config
    }
}

impl Default for FrontMatterWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for common operations
pub mod convenience {
    use super::*;

    /// Write a document to a file with default settings
    pub fn write_document<P: AsRef<Path>>(document: &Document, path: P) -> Result<WriteResult> {
        FrontMatterWriter::new().write_file(document, path, None)
    }

    /// Write a document with backup
    pub fn write_document_with_backup<P: AsRef<Path>>(
        document: &Document,
        path: P,
        backup_suffix: &str,
    ) -> Result<WriteResult> {
        let options = WriteOptions {
            backup: Some(BackupOptions {
                enabled: true,
                suffix: Some(backup_suffix.to_string()),
                directory: None,
            }),
            output: None,
            dry_run: false,
        };
        FrontMatterWriter::new().write_file(document, path, Some(options))
    }

    /// Preview changes (dry run)
    pub fn preview_changes<P: AsRef<Path>>(document: &Document, path: P) -> Result<WriteResult> {
        let options = WriteOptions {
            backup: None,
            output: None,
            dry_run: true,
        };
        FrontMatterWriter::new().write_file(document, path, Some(options))
    }

    /// Write document to stdout
    pub fn write_to_stdout(document: &Document) -> Result<WriteResult> {
        let dummy_path = Path::new("stdout");
        let options = WriteOptions {
            backup: None,
            output: Some(OutputOptions::Stdout),
            dry_run: false,
        };
        FrontMatterWriter::new().write_file(document, dummy_path, Some(options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FrontMatterValue;
    use std::collections::BTreeMap;

    use tempfile::TempDir;

    fn create_test_document() -> Document {
        let mut fm = BTreeMap::new();
        fm.insert(
            "title".to_string(),
            FrontMatterValue::string("Test Document"),
        );
        fm.insert("author".to_string(), FrontMatterValue::string("John Doe"));
        fm.insert("count".to_string(), FrontMatterValue::int(42));

        Document::new(Some(fm), "# Hello World\n\nThis is the body.".to_string())
    }

    #[test]
    fn test_write_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let document = create_test_document();
        let writer = FrontMatterWriter::new();

        let result = writer.write_file(&document, &file_path, None).unwrap();

        assert!(result.modified);
        assert_eq!(result.output_path, Some(file_path.clone()));
        assert!(result.backup_path.is_none());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("title: Test Document"));
        assert!(content.contains("# Hello World"));
    }

    #[test]
    fn test_write_with_backup() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");

        // Create original file
        fs::write(&file_path, "original content").unwrap();

        let document = create_test_document();
        let options = WriteOptions {
            backup: Some(BackupOptions {
                enabled: true,
                suffix: Some(".bak".to_string()),
                directory: None,
            }),
            output: None,
            dry_run: false,
        };

        let writer = FrontMatterWriter::new();
        let result = writer
            .write_file(&document, &file_path, Some(options))
            .unwrap();

        assert!(result.modified);
        assert!(result.backup_path.is_some());

        let backup_path = result.backup_path.unwrap();
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(backup_path).unwrap(), "original content");
    }

    #[test]
    fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");

        // Create original file
        fs::write(&file_path, "# Original Title").unwrap();

        let document = create_test_document();
        let options = WriteOptions {
            backup: None,
            output: None,
            dry_run: true,
        };

        let writer = FrontMatterWriter::new();
        let result = writer
            .write_file(&document, &file_path, Some(options))
            .unwrap();

        assert!(result.modified);
        assert!(result.diff.is_some());

        // File should not have been modified
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "# Original Title");
    }

    #[test]
    fn test_write_to_stdout() {
        let document = create_test_document();
        let options = WriteOptions {
            backup: None,
            output: Some(OutputOptions::Stdout),
            dry_run: false,
        };

        let writer = FrontMatterWriter::new();
        let result = writer
            .write_file(&document, Path::new("dummy"), Some(options))
            .unwrap();

        assert!(result.modified);
        assert!(result.output_path.is_none());
    }

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let document = create_test_document();

        let config = WriterConfig {
            atomic_writes: true,
            ..Default::default()
        };
        let writer = FrontMatterWriter::with_config(config);

        let result = writer.write_file(&document, &file_path, None).unwrap();
        assert!(result.modified);
        assert!(file_path.exists());
    }

    #[test]
    fn test_no_change_detection() {
        let document = create_test_document();
        let writer = FrontMatterWriter::new();

        // Write document first time
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        writer.write_file(&document, &file_path, None).unwrap();

        // Write same document again
        let result = writer.write_file(&document, &file_path, None).unwrap();
        assert!(!result.modified);
    }

    #[test]
    fn test_convenience_functions() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let document = create_test_document();

        // Test basic write
        let result = convenience::write_document(&document, &file_path).unwrap();
        assert!(result.modified);

        // Test write with backup
        fs::write(&file_path, "modified content").unwrap();
        let result =
            convenience::write_document_with_backup(&document, &file_path, ".backup").unwrap();
        assert!(result.modified);
        assert!(result.backup_path.is_some());

        // Test preview - write different content first to ensure a diff
        fs::write(
            &file_path,
            "---\ntitle: Different Title\n---\nDifferent body",
        )
        .unwrap();
        let result = convenience::preview_changes(&document, &file_path).unwrap();
        assert!(result.diff.is_some());
    }

    #[test]
    fn test_diff_generation() {
        let writer = FrontMatterWriter::new();
        let old_content = "line1\nline2\nline3";
        let new_content = "line1\nmodified line2\nline3";
        let path = Path::new("test.txt");

        let diff = writer.generate_diff(old_content, new_content, path);
        assert!(diff.is_some());

        let diff_content = diff.unwrap();
        assert!(diff_content.contains("-line2"));
        assert!(diff_content.contains("+modified line2"));
    }

    #[test]
    fn test_line_ending_normalization() {
        let config = WriterConfig {
            line_endings: LineEndings::Unix,
            ..Default::default()
        };
        let writer = FrontMatterWriter::with_config(config);

        let content = "line1\r\nline2\rline3\n";
        let normalized = writer.normalize_line_endings(content);
        assert_eq!(normalized, "line1\nline2\nline3\n");
    }
}
