//! Key path parsing and manipulation for nested front matter access
//!
//! This module provides a robust system for parsing and working with nested
//! key paths in front matter, supporting dot notation, bracket notation,
//! and proper escaping.

use crate::error::{MatterOfError, Result};
use std::fmt;

/// Represents a parsed key path for accessing nested values
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyPath {
    segments: Vec<String>,
}

impl KeyPath {
    /// Create a new empty key path
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Create a key path from a single segment
    pub fn single(key: impl Into<String>) -> Self {
        Self {
            segments: vec![key.into()],
        }
    }

    /// Create a key path from multiple segments
    pub fn from_segments(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// Parse a key path from a string
    ///
    /// Supports multiple formats:
    /// - Dot notation: "parent.child.key"
    /// - Bracket notation: "parent\['child'\]\['key'\]" or "parent\[\"child\"\]\[\"key\"\]"
    /// - Mixed notation: "parent.child['special.key']"
    /// - Escaped keys: "parent.\"key.with.dots\".child"
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Ok(Self::new());
        }

        let mut segments = Vec::new();
        let mut parser = PathParser::new(input);

        while !parser.is_at_end() {
            let segment = parser.parse_segment()?;
            if !segment.is_empty() {
                segments.push(segment);
            }
        }

        Ok(Self::from_segments(segments))
    }

    /// Get the segments of this path
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Get the number of segments
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the first segment (root key)
    pub fn first(&self) -> Option<&str> {
        self.segments.first().map(|s| s.as_str())
    }

    /// Get the last segment (leaf key)
    pub fn last(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Get a subpath from the given index
    pub fn subpath(&self, from: usize) -> Self {
        Self::from_segments(self.segments.get(from..).unwrap_or(&[]).to_vec())
    }

    /// Get a subpath up to the given index (exclusive)
    pub fn prefix(&self, to: usize) -> Self {
        Self::from_segments(self.segments.get(..to).unwrap_or(&[]).to_vec())
    }

    /// Append a segment to this path
    pub fn push(&mut self, segment: impl Into<String>) {
        self.segments.push(segment.into());
    }

    /// Append another path to this path
    pub fn extend(&mut self, other: &KeyPath) {
        self.segments.extend(other.segments.iter().cloned());
    }

    /// Create a new path by appending a segment
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut new_path = self.clone();
        new_path.push(segment);
        new_path
    }

    /// Create a new path by appending another path
    pub fn join(&self, other: &KeyPath) -> Self {
        let mut new_path = self.clone();
        new_path.extend(other);
        new_path
    }

    /// Check if this path starts with another path
    pub fn starts_with(&self, prefix: &KeyPath) -> bool {
        if prefix.len() > self.len() {
            return false;
        }

        self.segments
            .iter()
            .zip(prefix.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Check if this path is a parent of another path
    pub fn is_parent_of(&self, child: &KeyPath) -> bool {
        child.starts_with(self) && child.len() > self.len()
    }

    /// Convert to dot notation string
    pub fn to_dot_notation(&self) -> String {
        self.segments
            .iter()
            .map(|s| escape_key_for_dot_notation(s))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Convert to bracket notation string
    pub fn to_bracket_notation(&self) -> String {
        if self.segments.is_empty() {
            return String::new();
        }

        let mut result = self.segments[0].clone();
        for segment in &self.segments[1..] {
            result.push_str(&format!("[\"{}\"]", escape_string_for_brackets(segment)));
        }
        result
    }
}

impl Default for KeyPath {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_dot_notation())
    }
}

impl From<String> for KeyPath {
    fn from(s: String) -> Self {
        Self::parse(&s).unwrap_or_else(|_| Self::single(s))
    }
}

impl From<&str> for KeyPath {
    fn from(s: &str) -> Self {
        Self::parse(s).unwrap_or_else(|_| Self::single(s))
    }
}

impl From<Vec<String>> for KeyPath {
    fn from(segments: Vec<String>) -> Self {
        Self::from_segments(segments)
    }
}

/// Internal parser for key paths
struct PathParser<'a> {
    input: &'a str,
    chars: std::str::CharIndices<'a>,
    current: Option<(usize, char)>,
}

impl<'a> PathParser<'a> {
    fn new(input: &'a str) -> Self {
        let mut chars = input.char_indices();
        let current = chars.next();
        Self {
            input,
            chars,
            current,
        }
    }

    fn is_at_end(&self) -> bool {
        self.current.is_none()
    }

    fn current_char(&self) -> Option<char> {
        self.current.map(|(_, c)| c)
    }

    fn advance(&mut self) {
        self.current = self.chars.next();
    }

    #[allow(dead_code)]
    fn peek(&self) -> Option<char> {
        self.chars.as_str().chars().next()
    }

    fn parse_segment(&mut self) -> Result<String> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(String::new());
        }

        match self.current_char() {
            Some('[') => self.parse_bracket_segment(),
            Some('"') => self.parse_quoted_segment(),
            Some('\'') => self.parse_quoted_segment(),
            Some('.') => {
                self.advance(); // Skip the dot
                self.parse_segment()
            }
            _ => self.parse_unquoted_segment(),
        }
    }

    fn parse_bracket_segment(&mut self) -> Result<String> {
        self.advance(); // Skip '['
        self.skip_whitespace();

        let segment = match self.current_char() {
            Some('"') | Some('\'') => self.parse_quoted_segment()?,
            _ => self.parse_unquoted_bracket_content()?,
        };

        self.skip_whitespace();

        if self.current_char() != Some(']') {
            return Err(MatterOfError::invalid_key_path(
                self.input,
                "missing closing bracket",
            ));
        }

        self.advance(); // Skip ']'
        Ok(segment)
    }

    fn parse_quoted_segment(&mut self) -> Result<String> {
        let quote_char = self.current_char().unwrap();
        self.advance(); // Skip opening quote

        let mut result = String::new();
        let mut escaped = false;

        while let Some(ch) = self.current_char() {
            if escaped {
                match ch {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '\'' => result.push('\''),
                    '"' => result.push('"'),
                    _ => {
                        result.push('\\');
                        result.push(ch);
                    }
                }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                self.advance(); // Skip closing quote
                return Ok(result);
            } else {
                result.push(ch);
            }
            self.advance();
        }

        Err(MatterOfError::invalid_key_path(
            self.input,
            "unterminated quoted string",
        ))
    }

    fn parse_unquoted_segment(&mut self) -> Result<String> {
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            match ch {
                '.' | '[' => break,
                '\\' => {
                    self.advance();
                    if let Some(escaped_char) = self.current_char() {
                        result.push(escaped_char);
                        self.advance();
                    } else {
                        result.push('\\');
                    }
                }
                _ => {
                    result.push(ch);
                    self.advance();
                }
            }
        }

        Ok(result.trim().to_string())
    }

    fn parse_unquoted_bracket_content(&mut self) -> Result<String> {
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            match ch {
                ']' => break,
                '\\' => {
                    self.advance();
                    if let Some(escaped_char) = self.current_char() {
                        result.push(escaped_char);
                        self.advance();
                    } else {
                        result.push('\\');
                    }
                }
                _ => {
                    result.push(ch);
                    self.advance();
                }
            }
        }

        Ok(result.trim().to_string())
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
}

/// Escape a key for use in dot notation
fn escape_key_for_dot_notation(key: &str) -> String {
    if key.contains('.') || key.contains('\\') || key.contains('"') {
        format!("\"{}\"", escape_string_for_quotes(key))
    } else {
        key.to_string()
    }
}

/// Escape a string for use in double quotes
fn escape_string_for_quotes(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\t' => "\\t".to_string(),
            '\r' => "\\r".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Escape a string for use in brackets
fn escape_string_for_brackets(s: &str) -> String {
    escape_string_for_quotes(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dot_notation() {
        let path = KeyPath::parse("parent.child.key").unwrap();
        assert_eq!(path.segments(), &["parent", "child", "key"]);
        assert_eq!(path.to_dot_notation(), "parent.child.key");
    }

    #[test]
    fn test_bracket_notation() {
        let path = KeyPath::parse("parent['child']['key']").unwrap();
        assert_eq!(path.segments(), &["parent", "child", "key"]);
    }

    #[test]
    fn test_quoted_keys() {
        let path = KeyPath::parse("parent.\"key.with.dots\".child").unwrap();
        assert_eq!(path.segments(), &["parent", "key.with.dots", "child"]);
    }

    #[test]
    fn test_mixed_notation() {
        let path = KeyPath::parse("parent.child['special.key']").unwrap();
        assert_eq!(path.segments(), &["parent", "child", "special.key"]);
    }

    #[test]
    fn test_escaped_characters() {
        let path = KeyPath::parse("parent[\"key\\\"with\\\"quotes\"]").unwrap();
        assert_eq!(path.segments(), &["parent", "key\"with\"quotes"]);
    }

    #[test]
    fn test_empty_path() {
        let path = KeyPath::parse("").unwrap();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
    }

    #[test]
    fn test_single_segment() {
        let path = KeyPath::parse("key").unwrap();
        assert_eq!(path.segments(), &["key"]);
        assert_eq!(path.first(), Some("key"));
        assert_eq!(path.last(), Some("key"));
    }

    #[test]
    fn test_path_operations() {
        let path1 = KeyPath::parse("parent.child").unwrap();
        let path2 = KeyPath::parse("key").unwrap();

        let joined = path1.join(&path2);
        assert_eq!(joined.segments(), &["parent", "child", "key"]);

        let child_path = path1.child("newkey");
        assert_eq!(child_path.segments(), &["parent", "child", "newkey"]);
    }

    #[test]
    fn test_path_relationships() {
        let parent = KeyPath::parse("parent.child").unwrap();
        let child = KeyPath::parse("parent.child.key").unwrap();

        assert!(child.starts_with(&parent));
        assert!(parent.is_parent_of(&child));
        assert!(!parent.starts_with(&child));
    }

    #[test]
    fn test_subpath_operations() {
        let path = KeyPath::parse("a.b.c.d").unwrap();

        let subpath = path.subpath(1);
        assert_eq!(subpath.segments(), &["b", "c", "d"]);

        let prefix = path.prefix(2);
        assert_eq!(prefix.segments(), &["a", "b"]);
    }

    #[test]
    fn test_invalid_paths() {
        assert!(KeyPath::parse("parent['unterminated").is_err());
        assert!(KeyPath::parse("parent[\"unterminated").is_err());
        assert!(KeyPath::parse("parent[missing_bracket").is_err());
    }

    #[test]
    fn test_notation_conversion() {
        let path = KeyPath::parse("parent.child.\"key.with.dots\"").unwrap();

        let dot_notation = path.to_dot_notation();
        assert_eq!(dot_notation, "parent.child.\"key.with.dots\"");

        let bracket_notation = path.to_bracket_notation();
        assert_eq!(bracket_notation, "parent[\"child\"][\"key.with.dots\"]");
    }
}
