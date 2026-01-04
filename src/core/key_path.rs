use serde_yaml::Value;
use regex::Regex;

#[derive(Clone, Debug)]
pub enum KeyPathSegment {
    Key(String),
    Index(usize),
    Regex(Regex),
}

/// A path that can contain keys, indices, or regex patterns.
#[derive(Clone, Debug, Default)]
pub struct KeyPath(pub Vec<KeyPathSegment>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResolvedKeyPathSegment {
    Key(String),
    Index(usize),
}

/// A fully resolved, absolute path to a specific value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResolvedKeyPath(pub Vec<ResolvedKeyPathSegment>);

impl KeyPath {
    pub fn parse(raw: &str) -> Self {
        if raw.is_empty() { return Self(vec![]); }
        
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut in_brackets = false;
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '"' => in_quotes = !in_quotes,
                '[' if !in_quotes => {
                    if !current.is_empty() {
                        segments.push(KeyPathSegment::Key(current.clone()));
                        current.clear();
                    }
                    in_brackets = true;
                }
                ']' if !in_quotes && in_brackets => {
                    if !current.is_empty() {
                        let s = current.trim_matches('"');
                        if let Ok(idx) = s.parse::<usize>() {
                            segments.push(KeyPathSegment::Index(idx));
                        } else {
                            segments.push(KeyPathSegment::Key(s.to_string()));
                        }
                        current.clear();
                    }
                    in_brackets = false;
                }
                '.' if !in_quotes && !in_brackets => {
                    if !current.is_empty() {
                        segments.push(KeyPathSegment::Key(current.clone()));
                        current.clear();
                    }
                }
                _ => current.push(chars[i]),
            }
            i += 1;
        }
        if !current.is_empty() {
            segments.push(KeyPathSegment::Key(current));
        }
        Self(segments)
    }

    pub fn append_key(&mut self, key: String) {
        self.0.push(KeyPathSegment::Key(key));
    }

    pub fn append_regex(&mut self, regex: Regex) {
        self.0.push(KeyPathSegment::Regex(regex));
    }

    pub fn append_index(&mut self, index: usize) {
        self.0.push(KeyPathSegment::Index(index));
    }

    pub fn to_resolved(&self) -> Option<ResolvedKeyPath> {
        let mut res = Vec::new();
        for s in &self.0 {
            match s {
                KeyPathSegment::Key(k) => res.push(ResolvedKeyPathSegment::Key(k.clone())),
                KeyPathSegment::Index(i) => res.push(ResolvedKeyPathSegment::Index(*i)),
                KeyPathSegment::Regex(_) => return None,
            }
        }
        Some(ResolvedKeyPath(res))
    }
}

pub fn val_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => serde_yaml::to_string(val).unwrap_or_default().trim().trim_start_matches("---\
").trim().to_string(),
    }
}
