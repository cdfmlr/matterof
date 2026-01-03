use regex::Regex;
use crate::core::path::val_to_string;

#[derive(Default, Debug)]
pub struct Selector {
    pub keys: Vec<Vec<String>>,
    pub key_parts: Vec<String>,
    pub key_regex: Option<Regex>,
    pub key_part_regex: Vec<Regex>,
    pub value_match: Option<String>,
    pub value_regex: Option<Regex>,
    pub all: bool,
}

impl Selector {
    pub fn matches(&self, path: &[String], value: &serde_yaml::Value) -> bool {
        if self.all {
            return true;
        }

        let mut key_match = false;

        // 1. Explicit keys
        for k in &self.keys {
            if path.starts_with(k) {
                key_match = true;
                break;
            }
        }

        // 2. Key parts
        if !self.key_parts.is_empty() && path.starts_with(&self.key_parts) {
            key_match = true;
        }

        // 3. Key Regex
        if let Some(re) = &self.key_regex {
            if re.is_match(&path.join(".")) {
                key_match = true;
            }
        }

        // 4. Key Part Regex
        if !self.key_part_regex.is_empty() && path.len() >= self.key_part_regex.len() {
            let mut m = true;
            for (i, re) in self.key_part_regex.iter().enumerate() {
                if !re.is_match(&path[i]) {
                    m = false;
                    break;
                }
            }
            if m {
                key_match = true;
            }
        }

        if !key_match {
            return false;
        }

        // Value checks
        if let Some(v) = &self.value_match {
            if val_to_string(value) != *v {
                return false;
            }
        }

        if let Some(re) = &self.value_regex {
            if !re.is_match(&val_to_string(value)) {
                return false;
            }
        }

        true
    }
}
