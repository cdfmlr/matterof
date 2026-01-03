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
        _ => {} // Ignore other types for flattening
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
                 // If current is not a mapping, convert it to one if it's null
                 if current.is_null() {
                     *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                 }
                 // Now it must be a mapping (or we panic/error if it's not)
                 if let serde_yaml::Value::Mapping(map) = current {
                      map.insert(serde_yaml::Value::String(part.clone()), val);
                 } else {
                     // Handle error: expected a mapping but found something else
                     // For simplicity, we might overwrite or panic here depending on desired behavior
                     // For now, let's assume it should be a mapping and overwrite if not null
                     *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                     if let serde_yaml::Value::Mapping(map) = current {
                          map.insert(serde_yaml::Value::String(part.clone()), val);
                     }
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
                 // Ensure we get a mutable reference to the nested mapping
                 current = map.get_mut(&key).unwrap();
             } else {
                 // If current is not a mapping, convert it to one if it's null
                 if current.is_null() {
                     *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                 } else {
                     // If it's not null and not a mapping, we have a type conflict.
                     // For simplicity, overwrite with a new mapping.
                     *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                 }
                 // Now it must be a mapping
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
        // If source is a sequence and target is a sequence, concatenate them
        (serde_yaml::Value::Sequence(t_seq), serde_yaml::Value::Sequence(s_seq)) => {
            t_seq.extend(s_seq);
        }
        // Otherwise, overwrite target with source
        (t, s) => {
            *t = s;
        }
    }
}

pub fn val_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => String::new(), // Return empty string for other types like Sequence or Mapping
    }
}
