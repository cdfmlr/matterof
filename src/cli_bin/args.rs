//! Command-line argument definitions and parsing
//!
//! This module provides clean, well-structured CLI argument parsing using clap,
//! with proper separation between CLI concerns and library operations.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Main CLI application
#[derive(Parser)]
#[command(
    name = "matterof",
    version,
    about = "A command-line tool for manipulating YAML front matter in markdown files",
    long_about = "matterof is a powerful tool for reading, querying, modifying, and managing \
                  YAML front matter in markdown files. It supports complex queries, batch \
                  operations, atomic writes, and backup functionality."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Get front matter values
    Get(GetArgs),
    /// Set front matter values
    Set(SetArgs),
    /// Add values to arrays or create new entries
    Add(AddArgs),
    /// Remove keys or values
    Remove(RemoveArgs),
    /// Replace/rename keys or values
    Replace(ReplaceArgs),
    /// Initialize front matter in files
    Init(InitArgs),
    /// Remove empty front matter blocks
    Clean(CleanArgs),
    /// Validate front matter syntax
    Validate(ValidateArgs),
    /// Format front matter (sort keys, normalize formatting)
    Format(FormatArgs),
}

/// Common options for file operations
#[derive(Args, Debug, Clone)]
pub struct CommonFileOptions {
    /// Files or directories to process
    pub files: Vec<PathBuf>,

    /// Follow symbolic links when processing directories
    #[arg(long)]
    pub follow_links: bool,

    /// Maximum depth for directory recursion
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Include hidden files (starting with .)
    #[arg(long)]
    pub include_hidden: bool,

    /// Only process files with these extensions
    #[arg(long = "ext", value_name = "EXT")]
    pub extensions: Vec<String>,

    /// Exclude files matching these patterns
    #[arg(long = "exclude", value_name = "PATTERN")]
    pub exclude_patterns: Vec<String>,
}

/// Common options for write operations
#[derive(Args, Debug, Clone)]
pub struct WriteOptions {
    /// Preview changes without modifying files (show diff)
    #[arg(long)]
    pub dry_run: bool,

    /// Create backup files with this suffix
    #[arg(long, value_name = "SUFFIX")]
    pub backup_suffix: Option<String>,

    /// Create backup files in this directory
    #[arg(long, value_name = "DIR")]
    pub backup_dir: Option<PathBuf>,

    /// Output modified content to stdout instead of writing to file
    #[arg(long)]
    pub stdout: bool,

    /// Write modified files to this directory
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,

    /// Use atomic writes (default: true)
    #[arg(long)]
    pub no_atomic: bool,

    /// Line ending style
    #[arg(long, value_enum)]
    pub line_endings: Option<LineEndingStyle>,
}

/// Line ending styles for output
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum LineEndingStyle {
    /// Unix-style line endings (\n)
    Unix,
    /// Windows-style line endings (\r\n)
    Windows,
    /// Preserve original line endings
    Preserve,
}

/// Arguments for the get command
#[derive(Args, Debug)]
pub struct GetArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    /// Get all front matter keys and values
    #[arg(long, conflicts_with_all = ["key", "key_part", "key_regex"])]
    pub all: bool,

    /// Specific keys to get (supports dot notation and brackets)
    #[arg(short, long, value_name = "KEY")]
    pub key: Vec<String>,

    /// Key parts for building nested keys
    #[arg(long = "key-part", value_name = "PART")]
    pub key_part: Vec<String>,

    /// Regular expression to match keys
    #[arg(long = "key-regex", value_name = "REGEX")]
    pub key_regex: Option<String>,

    /// Regular expression to match key parts in nested paths
    #[arg(long = "key-part-regex", value_name = "REGEX")]
    pub key_part_regex: Vec<String>,

    /// Regular expression to match values
    #[arg(long = "value-regex", value_name = "REGEX")]
    pub value_regex: Option<String>,

    /// Exact value to match
    #[arg(long = "value", value_name = "VALUE")]
    pub value_match: Option<String>,

    /// Only return keys that exist (are not null)
    #[arg(long)]
    pub exists_only: bool,

    /// Only return keys at this depth
    #[arg(long, value_name = "DEPTH")]
    pub depth: Option<usize>,

    /// Output format
    #[arg(long, value_enum, default_value = "yaml")]
    pub format: OutputFormat,

    /// Pretty print output
    #[arg(long)]
    pub pretty: bool,
}

/// Arguments for the set command
#[derive(Args, Debug)]
pub struct SetArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Keys to set (supports dot notation and brackets)
    #[arg(short, long, value_name = "KEY", required_unless_present = "key_regex")]
    pub key: Vec<String>,

    /// Key parts for building nested keys
    #[arg(long = "key-part", value_name = "PART")]
    pub key_part: Vec<String>,

    /// Regular expression to match keys to set
    #[arg(long = "key-regex", value_name = "REGEX")]
    pub key_regex: Option<String>,

    /// Values to set
    #[arg(short = 'V', long, value_name = "VALUE", required = true)]
    pub value: Vec<String>,

    /// Value type for type conversion
    #[arg(short, long, value_enum)]
    pub type_: Option<ValueType>,

    /// Create intermediate keys if they don't exist
    #[arg(long)]
    pub create_parents: bool,
}

/// Arguments for the add command
#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Key where to add the value (will create array if doesn't exist)
    #[arg(short, long, value_name = "KEY", required_unless_present = "key_part")]
    pub key: Option<String>,

    /// Key parts for building nested key
    #[arg(long = "key-part", value_name = "PART")]
    pub key_part: Vec<String>,

    /// Value to add
    #[arg(short = 'V', long, value_name = "VALUE", required = true)]
    pub value: String,

    /// Value type for type conversion
    #[arg(short, long, value_enum)]
    pub type_: Option<ValueType>,

    /// Index to insert at (default: append to end)
    #[arg(long, value_name = "INDEX")]
    pub index: Option<usize>,
}

/// Arguments for the remove command
#[derive(Args, Debug)]
pub struct RemoveArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Remove all front matter
    #[arg(long, conflicts_with_all = ["key", "key_part", "key_regex", "value", "value_regex"])]
    pub all: bool,

    /// Keys to remove
    #[arg(short, long, value_name = "KEY")]
    pub key: Vec<String>,

    /// Key parts for building nested keys to remove
    #[arg(long = "key-part", value_name = "PART")]
    pub key_part: Vec<String>,

    /// Regular expression to match keys to remove
    #[arg(long = "key-regex", value_name = "REGEX")]
    pub key_regex: Option<String>,

    /// Specific value to remove from arrays/objects
    #[arg(long = "value", value_name = "VALUE")]
    pub value: Option<String>,

    /// Regular expression to match values to remove
    #[arg(long = "value-regex", value_name = "REGEX")]
    pub value_regex: Option<String>,

    /// Remove empty parent objects after removal
    #[arg(long)]
    pub cleanup_empty: bool,
}

/// Arguments for the replace command
#[derive(Args, Debug)]
pub struct ReplaceArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Keys to replace/rename
    #[arg(short, long, value_name = "KEY")]
    pub key: Vec<String>,

    /// Key parts for building nested keys to replace
    #[arg(long = "key-part", value_name = "PART")]
    pub key_part: Vec<String>,

    /// Regular expression to match keys to replace
    #[arg(long = "key-regex", value_name = "REGEX")]
    pub key_regex: Option<String>,

    /// New key name (for renaming keys)
    #[arg(long = "new-key", value_name = "KEY")]
    pub new_key: Option<String>,

    /// New key parts for building nested new key
    #[arg(long = "new-key-part", value_name = "PART")]
    pub new_key_part: Vec<String>,

    /// New value to set (replaces old value)
    #[arg(long = "new-value", value_name = "VALUE")]
    pub new_value: Option<String>,

    /// Old value to replace (when replacing specific values)
    #[arg(long = "old-value", value_name = "VALUE")]
    pub old_value: Option<String>,

    /// Regular expression to match old values to replace
    #[arg(long = "old-value-regex", value_name = "REGEX")]
    pub old_value_regex: Option<String>,

    /// Value type for type conversion of new value
    #[arg(short, long, value_enum)]
    pub type_: Option<ValueType>,
}

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Initialize with these default key-value pairs
    #[arg(long = "default", value_name = "KEY=VALUE")]
    pub defaults: Vec<String>,

    /// Only initialize files that don't have front matter
    #[arg(long)]
    pub only_missing: bool,
}

/// Arguments for the clean command
#[derive(Args, Debug)]
pub struct CleanArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Remove front matter blocks that are empty or contain only null values
    #[arg(long)]
    pub remove_null: bool,
}

/// Arguments for the validate command
#[derive(Args, Debug)]
pub struct ValidateArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    /// Exit with non-zero code on first validation error
    #[arg(long)]
    pub fail_fast: bool,

    /// Output format for validation results
    #[arg(long, value_enum, default_value = "human")]
    pub format: ValidationFormat,
}

/// Arguments for the format command
#[derive(Args, Debug)]
pub struct FormatArgs {
    #[command(flatten)]
    pub files: CommonFileOptions,

    #[command(flatten)]
    pub write_options: WriteOptions,

    /// Sort keys alphabetically
    #[arg(long)]
    pub sort_keys: bool,

    /// Indentation level for YAML output
    #[arg(long, value_name = "SPACES", default_value = "2")]
    pub indent: usize,

    /// Remove null values
    #[arg(long)]
    pub remove_null: bool,
}

/// Value types for type conversion
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ValueType {
    /// String value
    String,
    /// Integer value
    Int,
    /// Floating point value
    Float,
    /// Boolean value
    Bool,
    /// Array/sequence value
    Array,
    /// Object/mapping value
    Object,
}

/// Output formats for get command
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OutputFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
    /// Plain text (values only)
    Text,
    /// CSV format (for tabular data)
    Csv,
}

/// Output formats for validate command
#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ValidationFormat {
    /// Human-readable format
    Human,
    /// JSON format
    Json,
    /// Simple format (just file paths)
    Simple,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            backup_suffix: None,
            backup_dir: None,
            stdout: false,
            output_dir: None,
            no_atomic: false,
            line_endings: None,
        }
    }
}

impl Default for CommonFileOptions {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            follow_links: false,
            max_depth: None,
            include_hidden: false,
            extensions: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

impl From<LineEndingStyle> for matterof::io::LineEndings {
    fn from(style: LineEndingStyle) -> Self {
        match style {
            LineEndingStyle::Unix => Self::Unix,
            LineEndingStyle::Windows => Self::Windows,
            LineEndingStyle::Preserve => Self::Preserve,
        }
    }
}

impl From<ValueType> for matterof::core::ValueType {
    fn from(vt: ValueType) -> Self {
        match vt {
            ValueType::String => Self::String,
            ValueType::Int => Self::Int,
            ValueType::Float => Self::Float,
            ValueType::Bool => Self::Bool,
            ValueType::Array => Self::Array,
            ValueType::Object => Self::Object,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI can be parsed without errors
        Cli::command().debug_assert();
    }

    #[test]
    fn test_get_command() {
        let args = vec!["matterof", "get", "--key", "title", "file.md"];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Commands::Get(get_args) = cli.command {
            assert_eq!(get_args.key, vec!["title"]);
            assert_eq!(get_args.files.files, vec![PathBuf::from("file.md")]);
            assert!(!get_args.all);
        } else {
            panic!("Expected Get command");
        }
    }

    #[test]
    fn test_set_command() {
        let args = vec![
            "matterof", "set", "--key", "title", "--value", "Hello", "file.md",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Commands::Set(set_args) = cli.command {
            assert_eq!(set_args.key, vec!["title"]);
            assert_eq!(set_args.value, vec!["Hello"]);
            assert_eq!(set_args.files.files, vec![PathBuf::from("file.md")]);
        } else {
            panic!("Expected Set command");
        }
    }

    #[test]
    fn test_write_options() {
        let args = vec![
            "matterof",
            "set",
            "--key",
            "title",
            "--value",
            "Hello",
            "--dry-run",
            "--backup-suffix",
            ".bak",
            "file.md",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Commands::Set(set_args) = cli.command {
            assert!(set_args.write_options.dry_run);
            assert_eq!(
                set_args.write_options.backup_suffix,
                Some(".bak".to_string())
            );
        } else {
            panic!("Expected Set command");
        }
    }

    #[test]
    fn test_file_options() {
        let args = vec![
            "matterof",
            "get",
            "--all",
            "--follow-links",
            "--max-depth",
            "3",
            "--include-hidden",
            "--ext",
            "md",
            "--ext",
            "markdown",
            "docs/",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        if let Commands::Get(get_args) = cli.command {
            assert!(get_args.files.follow_links);
            assert_eq!(get_args.files.max_depth, Some(3));
            assert!(get_args.files.include_hidden);
            assert_eq!(
                get_args.files.extensions,
                vec!["md".to_string(), "markdown".to_string()]
            );
        } else {
            panic!("Expected Get command");
        }
    }
}
