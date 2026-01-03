use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use gray_matter::Matter;
use gray_matter::engine::YAML;
use crate::args::CommonOpts;
use std::io::Write;

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map(|s| s == "md" || s == "markdown")
        .unwrap_or(false)
}

pub fn resolve_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && is_markdown(entry.path()) {
                    files.push(entry.path().to_owned());
                }
            }
        }
    }
    files
}

pub fn read_front_matter(path: &Path) -> Result<(Option<serde_yaml::Value>, String)> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read file {}", path.display()))?;
    
    // Check if it starts with ---
    if !content.trim_start().starts_with("---") {
        return Ok((None, content));
    }

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);
    
    let data = if let Some(d) = parsed.data {
        let val: serde_yaml::Value = d.deserialize()
            .with_context(|| format!("failed to deserialize front matter from {}", path.display()))?;
        Some(val)
    } else {
        None
    };

    Ok((data, parsed.content))
}

pub fn parse_key_path(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                in_quote = !in_quote;
            }
            '.' if !in_quote => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            '[' => {
                 if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            ']' => {
                 if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    
    parts.into_iter().filter(|s| !s.is_empty()).collect()
}

pub fn flatten_yaml(val: &serde_yaml::Value) -> Vec<(Vec<String>, serde_yaml::Value)> {
    let mut entries = Vec::new();
    flatten_recursive(val, &mut Vec::new(), &mut entries);
    entries
}

fn flatten_recursive(
    val: &serde_yaml::Value, 
    current_path: &mut Vec<String>, 
    entries: &mut Vec<(Vec<String>, serde_yaml::Value)>
) {
    if !current_path.is_empty() {
        entries.push((current_path.clone(), val.clone()));
    }

    match val {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                 if let Some(k_str) = k.as_str() {
                     current_path.push(k_str.to_string());
                     flatten_recursive(v, current_path, entries);
                     current_path.pop();
                 }
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for (i, v) in seq.iter().enumerate() {
                current_path.push(i.to_string());
                flatten_recursive(v, current_path, entries);
                current_path.pop();
            }
        }
        _ => {}
    }
}

pub fn unflatten_yaml(entries: Vec<(Vec<String>, serde_yaml::Value)>) -> serde_yaml::Value {
    let mut root = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    
    for (path, val) in entries {
        insert_at_path(&mut root, &path, val);
    }
    
    root
}

pub fn insert_at_path(root: &mut serde_yaml::Value, path: &[String], val: serde_yaml::Value) {
    if path.is_empty() {
        deep_merge(root, val);
        return;
    }

    let mut current = root;
    for (i, part) in path.iter().enumerate() {
        if i == path.len() - 1 {
            // Last part, insert value
            if let serde_yaml::Value::Mapping(map) = current {
                 let key = serde_yaml::Value::String(part.clone());
                 if let Some(existing) = map.get_mut(&key) {
                     deep_merge(existing, val);
                 } else {
                     map.insert(key, val);
                 }
            } else {
                 if current.is_null() {
                     *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                 }
                 if let serde_yaml::Value::Mapping(map) = current {
                      map.insert(serde_yaml::Value::String(part.clone()), val);
                 }
            }
            return;
        } else {
            // Intermediate part
            if current.is_null() {
                *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            }
            
             if let serde_yaml::Value::Mapping(map) = current {
                 let key = serde_yaml::Value::String(part.clone());
                 if !map.contains_key(&key) {
                     map.insert(key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
                 }
                 current = map.get_mut(&key).unwrap();
             } else {
                 *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                 if let serde_yaml::Value::Mapping(map) = current {
                    let key = serde_yaml::Value::String(part.clone());
                    map.insert(key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
                    current = map.get_mut(&key).unwrap();
                 }
             }
        }
    }
}

fn deep_merge(target: &mut serde_yaml::Value, source: serde_yaml::Value) {
    match (target, source) {
        (serde_yaml::Value::Mapping(t_map), serde_yaml::Value::Mapping(s_map)) => {
            for (k, v) in s_map {
                if let Some(existing) = t_map.get_mut(&k) {
                    deep_merge(existing, v);
                } else {
                    t_map.insert(k, v);
                }
            }
        }
        (t, s) => {
            *t = s;
        }
    }
}

pub fn write_result(file: &PathBuf, fm: Option<&serde_yaml::Value>, content: &str, opts: &CommonOpts) -> Result<()> {
    let new_content = if let Some(fm_val) = fm {
        let fm_str = serde_yaml::to_string(fm_val)?;
        let body = fm_str.trim_start_matches("---").trim();
        // Ensure body is not empty if map is not empty, but if map is empty "{}"?
        // if fm_val is empty map/null, body might be "{}" or "null" or empty.
        // If empty map, we want "---" "---" ?
        // Or "---" "{}" "---"?
        // If we want empty FM block, it's fine.
        format!("---\n{}\n---\n{}", body, content)
    } else {
        content.to_string()
    };
    
    if opts.dry_run {
         return show_diff(file, &new_content);
    }

    if opts.stdout {
        println!("{}", new_content);
        return Ok(())
    }

    // Backup
    if let Some(suffix) = &opts.backup_suffix {
        let mut name = file.file_name().unwrap().to_os_string();
        name.push(suffix);
        let backup_path = file.with_file_name(name);
        fs::copy(file, backup_path)?;
    }
    
    if let Some(dir) = &opts.backup_dir {
        let current_dir = std::env::current_dir()?;
        let rel = file.strip_prefix(&current_dir).unwrap_or(file);
        let target = dir.join(rel);
        if let Some(p) = target.parent() {
            fs::create_dir_all(p)?;
        }
        fs::copy(file, target)?;
    }

    if let Some(dir) = &opts.output_dir {
         let current_dir = std::env::current_dir()?;
         let rel = file.strip_prefix(&current_dir).unwrap_or(file);
         let target = dir.join(rel);
         if let Some(p) = target.parent() {
            fs::create_dir_all(p)?;
        }
         fs::write(target, new_content)?;
    } else {
         // In-place
         fs::write(file, new_content)?;
    }

    Ok(())
}

pub fn show_diff(path: &Path, new_content: &str) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new()?;
    write!(tmp, "{}", new_content)?;
    
    let output = std::process::Command::new("diff")
        .arg("-u")
        .arg(path)
        .arg(tmp.path())
        .output();

    match output {
        Ok(o) => {
            std::io::stdout().write_all(&o.stdout)?;
        },
        Err(_) => {
            println!("(diff command failed, showing new content)");
            println!("{}", new_content);
        }
    }
    Ok(())
}