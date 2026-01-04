//! Error types for the matterof library
//!
//! This module provides comprehensive error handling for all library operations,
//! including file I/O, YAML parsing, path resolution, and validation errors.

use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// The main error type for all library operations
#[derive(Error, Debug)]
pub enum MatterOfError {
    /// I/O related errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing or serialization errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Regular expression errors
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// File not found or invalid path
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Invalid file format (not a markdown file, etc.)
    #[error("Invalid file format: {path} (expected markdown)")]
    InvalidFileFormat { path: PathBuf },

    /// Front matter parsing errors
    #[error("Invalid front matter in {path}: {reason}")]
    InvalidFrontMatter { path: PathBuf, reason: String },

    /// Key path parsing errors
    #[error("Invalid key path: {path} ({reason})")]
    InvalidKeyPath { path: String, reason: String },

    /// Query errors
    #[error("Invalid query: {reason}")]
    InvalidQuery { reason: String },

    /// Value type conversion errors
    #[error("Type conversion error: cannot convert {from} to {to}")]
    TypeConversion { from: String, to: String },

    /// Path resolution errors
    #[error("Path resolution error: {reason}")]
    PathResolution { reason: String },

    /// Backup operation errors
    #[error("Backup error: {reason}")]
    BackupError { reason: String },

    /// Permission errors
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    /// File is locked or in use
    #[error("File is locked: {path}")]
    FileLocked { path: PathBuf },

    /// Operation not supported
    #[error("Operation not supported: {operation}")]
    NotSupported { operation: String },

    /// Generic validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Multiple errors (for batch operations)
    #[error("Multiple errors occurred")]
    Multiple { errors: Vec<MatterOfError> },
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, MatterOfError>;

impl MatterOfError {
    /// Create a new file not found error
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a new invalid file format error
    pub fn invalid_file_format(path: impl Into<PathBuf>) -> Self {
        Self::InvalidFileFormat { path: path.into() }
    }

    /// Create a new invalid front matter error
    pub fn invalid_front_matter(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::InvalidFrontMatter {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a new invalid key path error
    pub fn invalid_key_path(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidKeyPath {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a new invalid query error
    pub fn invalid_query(reason: impl Into<String>) -> Self {
        Self::InvalidQuery {
            reason: reason.into(),
        }
    }

    /// Create a new type conversion error
    pub fn type_conversion(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::TypeConversion {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a new path resolution error
    pub fn path_resolution(reason: impl Into<String>) -> Self {
        Self::PathResolution {
            reason: reason.into(),
        }
    }

    /// Create a new backup error
    pub fn backup_error(reason: impl Into<String>) -> Self {
        Self::BackupError {
            reason: reason.into(),
        }
    }

    /// Create a new permission denied error
    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    /// Create a new file locked error
    pub fn file_locked(path: impl Into<PathBuf>) -> Self {
        Self::FileLocked { path: path.into() }
    }

    /// Create a new not supported error
    pub fn not_supported(operation: impl Into<String>) -> Self {
        Self::NotSupported {
            operation: operation.into(),
        }
    }

    /// Create a new validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a multiple errors wrapper
    pub fn multiple(errors: Vec<MatterOfError>) -> Self {
        Self::Multiple { errors }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Io(io_err) => match io_err.kind() {
                std::io::ErrorKind::NotFound
                | std::io::ErrorKind::PermissionDenied
                | std::io::ErrorKind::AlreadyExists => false,
                _ => true,
            },
            Self::FileNotFound { .. }
            | Self::PermissionDenied { .. }
            | Self::NotSupported { .. } => false,
            Self::InvalidFileFormat { .. }
            | Self::InvalidFrontMatter { .. }
            | Self::InvalidKeyPath { .. }
            | Self::InvalidQuery { .. }
            | Self::TypeConversion { .. }
            | Self::PathResolution { .. }
            | Self::BackupError { .. }
            | Self::FileLocked { .. }
            | Self::Validation { .. } => true,
            Self::Yaml(_) | Self::Regex(_) => true,
            Self::Multiple { errors } => errors.iter().any(|e| e.is_recoverable()),
        }
    }

    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FileNotFound { .. } | Self::PermissionDenied { .. } => ErrorSeverity::Critical,
            Self::InvalidFrontMatter { .. } | Self::Yaml(_) => ErrorSeverity::High,
            Self::InvalidKeyPath { .. }
            | Self::InvalidQuery { .. }
            | Self::TypeConversion { .. } => ErrorSeverity::Medium,
            Self::Validation { .. } | Self::PathResolution { .. } => ErrorSeverity::Low,
            Self::Multiple { errors } => errors
                .iter()
                .map(|e| e.severity())
                .max()
                .unwrap_or(ErrorSeverity::Low),
            _ => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl Clone for MatterOfError {
    fn clone(&self) -> Self {
        match self {
            Self::Io(io_err) => {
                // Convert to a simple IO error message since std::io::Error doesn't implement Clone
                Self::Io(std::io::Error::new(io_err.kind(), io_err.to_string()))
            }
            Self::Yaml(_) => {
                // Create a new YAML error with a generic message since serde_yaml::Error doesn't implement Clone
                Self::Yaml(serde_yaml::from_str::<serde_yaml::Value>("invalid").unwrap_err())
            }
            Self::Regex(regex_err) => Self::Regex(regex_err.clone()),
            Self::FileNotFound { path } => Self::FileNotFound { path: path.clone() },
            Self::InvalidFileFormat { path } => Self::InvalidFileFormat { path: path.clone() },
            Self::InvalidFrontMatter { path, reason } => Self::InvalidFrontMatter {
                path: path.clone(),
                reason: reason.clone(),
            },
            Self::InvalidKeyPath { path, reason } => Self::InvalidKeyPath {
                path: path.clone(),
                reason: reason.clone(),
            },
            Self::InvalidQuery { reason } => Self::InvalidQuery {
                reason: reason.clone(),
            },
            Self::TypeConversion { from, to } => Self::TypeConversion {
                from: from.clone(),
                to: to.clone(),
            },
            Self::PathResolution { reason } => Self::PathResolution {
                reason: reason.clone(),
            },
            Self::BackupError { reason } => Self::BackupError {
                reason: reason.clone(),
            },
            Self::PermissionDenied { path } => Self::PermissionDenied { path: path.clone() },
            Self::FileLocked { path } => Self::FileLocked { path: path.clone() },
            Self::NotSupported { operation } => Self::NotSupported {
                operation: operation.clone(),
            },
            Self::Validation { message } => Self::Validation {
                message: message.clone(),
            },
            Self::Multiple { errors } => Self::Multiple {
                errors: errors.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = MatterOfError::file_not_found("test.md");
        assert!(matches!(err, MatterOfError::FileNotFound { .. }));
        assert!(!err.is_recoverable());
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::High);
        assert!(ErrorSeverity::High > ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
    }

    #[test]
    fn test_multiple_errors_severity() {
        let errors = vec![
            MatterOfError::validation("test"),
            MatterOfError::file_not_found("test.md"),
        ];
        let multi_err = MatterOfError::multiple(errors);
        assert_eq!(multi_err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_cloning() {
        let original = MatterOfError::file_not_found("test.md");
        let cloned = original.clone();

        match (&original, &cloned) {
            (
                MatterOfError::FileNotFound { path: p1 },
                MatterOfError::FileNotFound { path: p2 },
            ) => {
                assert_eq!(p1, p2);
            }
            _ => panic!("Cloned error doesn't match original"),
        }
    }
}
