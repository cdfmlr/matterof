use crate::args::*;
use crate::utils::*;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use regex::Regex;

pub fn run_get(args: GetArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    let mut results = HashMap::new();

    let key_regex = args.key_regex.as_deref().map(Regex::new).transpose()?;
    let value_regex = args.value_regex.as_deref().map(Regex::new).transpose()?;
    
    let key_part_regexes: Result<Vec<Regex>> = args.key_part_regex.iter()
        .map(|s| Regex::new(s).map_err(Into::into))
        .collect();
    let key_part_regexes = key_part_regexes?;

    for file in &files {
        let (front_matter, _) = read_front_matter(file)?;
        
        if let Some(fm) = front_matter {
             let extracted = extract_values(&fm, &args, &key_regex, &value_regex, &key_part_regexes)?;
             if !extracted.is_null() {
                 if let serde_yaml::Value::Mapping(m) = &extracted {
                     if !m.is_empty() {
                         results.insert(file.to_string_lossy().to_string(), extracted);
                     }
                 } else {
                     results.insert(file.to_string_lossy().to_string(), extracted);
                 }
             }
        }
    }
    
    if results.is_empty() {
        return Ok(());
    }

    if files.len() == 1 {
        let val = results.values().next().unwrap();
        let s = serde_yaml::to_string(val)?;
        println!("{}", s.trim_start_matches("---\
"));
    } else {
        let s = serde_yaml::to_string(&results)?;
        println!("{}", s.trim_start_matches("---\
"));
    }

    Ok(())
}

fn extract_values(
    fm: &serde_yaml::Value,
    args: &GetArgs,
    key_regex: &Option<Regex>,
    value_regex: &Option<Regex>,
    key_part_regexes: & [Regex]
) -> Result<serde_yaml::Value> {
    if args.all {
        return Ok(fm.clone());
    }

    let flattened = flatten_yaml(fm);
    let mut kept_entries = Vec::new();
    
    let explicit_keys: Vec<Vec<String>> = args.key.iter().map(|k| parse_key_path(k)).collect();
    
    for (path, val) in flattened {
        let mut key_match = false;
        
        // 1. Check explicit keys
        for k in &explicit_keys {
            if path.starts_with(k) {
                key_match = true;
                break;
            }
        }
        
        // 2. Check key-part (exact path segment sequence)
        if !args.key_part.is_empty() {
             if path.starts_with(&args.key_part) {
                 key_match = true;
             }
        }
        
        // 3. Check key-regex
        if let Some(re) = key_regex {
            let path_str = path.join(".");
            if re.is_match(&path_str) {
                key_match = true;
            }
        }

        // 4. Check key-part-regex
        if !key_part_regexes.is_empty() {
             if path.len() >= key_part_regexes.len() {
                 let mut m = true;
                 for (i, re) in key_part_regexes.iter().enumerate() {
                     if !re.is_match(&path[i]) {
                         m = false;
                         break;
                     }
                 }
                 if m {
                     key_match = true;
                 }
             }
        }

        if !key_match {
            continue;
        }

        // Value Check
        if let Some(re) = value_regex {
            let s = val_to_string(&val);
            if !re.is_match(&s) {
                continue;
            }
        }
        
        kept_entries.push((path, val));
    }
    
    Ok(unflatten_yaml(kept_entries))
}

fn val_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => String::new(), 
    }
}

pub fn run_set(args: SetArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    let values = parse_values(&args.value, &args.type_) ?;
    let new_val = if values.len() == 1 {
        values[0].clone()
    } else {
        serde_yaml::Value::Sequence(values)
    };

    let key_regex = args.key_regex.as_deref().map(Regex::new).transpose()?;

    for file in &files {
        let (fm_opt, content) = read_front_matter(file)?;
        let mut fm = fm_opt.unwrap_or_else(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

        if let Some(re) = &key_regex {
             let flattened = flatten_yaml(&fm);
             let mut paths_to_update = Vec::new();
             for (path, _) in flattened {
                  let path_str = path.join(".");
                  if re.is_match(&path_str) {
                      paths_to_update.push(path);
                  }
             }
             for path in paths_to_update {
                 insert_at_path(&mut fm, &path, new_val.clone());
             }
        } else {
             for k in &args.key {
                 let path = parse_key_path(k);
                 insert_at_path(&mut fm, &path, new_val.clone());
             }
             if !args.key_part.is_empty() {
                  insert_at_path(&mut fm, &args.key_part, new_val.clone());
             }
        }
        
        write_result(file, Some(&fm), &content, &args.opts)?;
    }
    Ok(())
}

fn parse_values(raw: &[String], type_: &Option<String>) -> Result<Vec<serde_yaml::Value>> {
    let mut vals = Vec::new();
    for r in raw {
        if raw.len() == 1 {
             for p in r.split(',') {
                 vals.push(parse_value(p, type_)?);
             }
        } else {
             vals.push(parse_value(r, type_)?);
        }
    }
    Ok(vals)
}

fn parse_value(v: &str, type_: &Option<String>) -> Result<serde_yaml::Value> {
    match type_.as_deref() {
        Some("int") => {
            let n: i64 = v.trim().parse()?;
            Ok(serde_yaml::Value::Number(n.into()))
        }
        Some("float") => {
            let n: f64 = v.trim().parse()?;
            Ok(serde_yaml::Value::Number(serde_yaml::Number::from(n)))
        }
        Some("bool") => {
            let b: bool = v.trim().parse()?;
            Ok(serde_yaml::Value::Bool(b))
        }
        Some("string") | None => {
            Ok(serde_yaml::Value::String(v.to_string()))
        }
        Some(t) => Err(anyhow!("Unknown type: {}", t)),
    }
}

pub fn run_add(args: AddArgs) -> Result<()> {
    // Append to list or create list
    let files = resolve_files(&args.files);
    // Value parsing same as set but usually single value? 
    // "add multiple values" not explicitly documented but maybe supported if passed?
    // "matterof add --key=key --value=value <file>"
    // Assuming single value for add, or reuse parse_values.
    let val = parse_value(&args.value, &None)?;

    for file in &files {
        let (fm_opt, content) = read_front_matter(file)?;
        if let Some(mut fm) = fm_opt {
             let path = if let Some(k) = &args.key {
                 parse_key_path(k)
             } else {
                 args.key_part.clone()
             };
             
             if path.is_empty() { continue; } // Need target

             // We need to find the node at path.
             // If node doesn't exist, create sequence [val]
             // If node is sequence, push val
             // If node is scalar/map, ?
             
             // This requires navigating mutable pointer or using flatten?
             // Since we modify specific path, navigation is better.
             // But I don't have navigate_mut helper easily exposed.
             
             // I'll use insert_at_path logic but modified to Append.
             // Or extract, modify, insert back.
             
             // Simple: flatten, find target, modify value, unflatten.
             // But modifying a Sequence in flatten view is tricky (keys are indices).
             
             // Implementation of deep navigate:
             // I'll implement a helper `modify_at_path` that takes a closure.
             
             modify_at_path(&mut fm, &path, |v| {
                 if v.is_null() {
                     *v = serde_yaml::Value::Sequence(vec![val.clone()]);
                 } else if let serde_yaml::Value::Sequence(seq) = v {
                     if let Some(idx) = args.index {
                         if idx <= seq.len() {
                             seq.insert(idx, val.clone());
                         } else {
                             seq.push(val.clone());
                         }
                     } else {
                         seq.push(val.clone());
                     }
                 } else {
                     // Convert scalar to list?
                     // "matterof add" implies adding to a collection.
                     // If I add to "hello", do I get ["hello", "val"]?
                     // Preserving existing value is good practice.
                     let old = v.clone();
                     *v = serde_yaml::Value::Sequence(vec![old, val.clone()]);
                 }
             });
             
             write_result(file, Some(&fm), &content, &args.opts)?;
        }
    }
    Ok(())
}

fn modify_at_path<F>(root: &mut serde_yaml::Value, path: &[String], f: F) 
where F: FnOnce(&mut serde_yaml::Value) {
    let mut current = root;
    for part in path {
        if current.is_null() {
             *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }
        if let serde_yaml::Value::Mapping(map) = current {
            let key = serde_yaml::Value::String(part.clone());
            if !map.contains_key(&key) {
                map.insert(key.clone(), serde_yaml::Value::Null);
            }
            current = map.get_mut(&key).unwrap();
        } else {
             // Navigate into scalar? Turn into map?
             // Overwrite scalar with map containing next key?
             // If we are navigating TO the target, we need path segments to be maps.
             *current = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
             if let serde_yaml::Value::Mapping(map) = current {
                let key = serde_yaml::Value::String(part.clone());
                map.insert(key.clone(), serde_yaml::Value::Null);
                current = map.get_mut(&key).unwrap();
             }
        }
    }
    f(current);
}

pub fn run_rm(args: RmArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    let key_regex = args.key_regex.as_deref().map(Regex::new).transpose()?;
    let value_regex = args.value_regex.as_deref().map(Regex::new).transpose()?;

    for file in &files {
        let (fm_opt, content) = read_front_matter(file)?;
        if args.all {
            write_result(file, None, &content, &args.opts)?;
            continue;
        }

        if let Some(fm) = fm_opt {
            let flattened = flatten_yaml(&fm);
            let mut kept = Vec::new();
            
            for (path, val) in flattened {
                let mut should_remove = false;
                
                if let Some(k_str) = &args.key {
                     let k_path = parse_key_path(k_str);
                     if path.starts_with(&k_path) {
                         should_remove = true;
                     }
                }
                
                if !args.key_part.is_empty() {
                    if path.starts_with(&args.key_part) {
                        should_remove = true;
                    }
                }

                if let Some(re) = &key_regex {
                    if re.is_match(&path.join(".")) {
                        should_remove = true;
                    }
                }

                if !should_remove {
                    kept.push((path, val));
                    continue;
                }

                let has_val_filter = args.value.is_some() || value_regex.is_some();

                if has_val_filter && (val.is_mapping() || val.is_sequence()) {
                    continue;
                }

                if let Some(v_target) = &args.value {
                    let s = val_to_string(&val);
                    if s != *v_target {
                         kept.push((path, val));
                         continue;
                    }
                }

                if let Some(re) = &value_regex {
                    let s = val_to_string(&val);
                    if !re.is_match(&s) {
                         kept.push((path, val));
                         continue;
                    }
                }
            }
            
            let new_fm = unflatten_yaml(kept);
            write_result(file, Some(&new_fm), &content, &args.opts)?;
        }
    }
    Ok(())
}

pub fn run_replace(args: ReplaceArgs) -> Result<()> {
    // Alias to set?
    if args.value.is_some() {
        // Construct SetArgs and run_set?
        // But ReplaceArgs has old-value checking.
        // And replace can rename key.
    }
    
    let files = resolve_files(&args.files);
    let key_regex = args.key_regex.as_deref().map(Regex::new).transpose()?;
    let old_value_regex = args.old_value_regex.as_deref().map(Regex::new).transpose()?;

    let new_val_parsed = if let Some(v) = &args.new_value {
        Some(parse_value(v, &args.type_) ?)
    } else { None };
    
    // Alias set if just value replacement without checks
    let simple_set = args.value.is_some();
    if simple_set {
        // Delegate to set logic or implement simply
        // "matterof replace --key=key [--type=?] --value=new_value <file>"
        // Same as set.
    }

    for file in &files {
        let (fm_opt, content) = read_front_matter(file)?;
        if let Some(fm) = fm_opt {
            let flattened = flatten_yaml(&fm);
            let mut new_entries = Vec::new();
            
            for (mut path, mut val) in flattened {
                let mut key_match = false;
                
                // Identify if this entry matches target key
                if let Some(k_str) = &args.key {
                     let k_path = parse_key_path(k_str);
                     if path.starts_with(&k_path) {
                         key_match = true;
                         // Check renaming
                         // Calculate new prefix
                         let mut nk_path = args.new_key_part.clone();
                         if let Some(nk) = &args.new_key {
                             nk_path.extend(parse_key_path(nk));
                         }
                         
                         if !nk_path.is_empty() {
                             // Replace prefix
                             // path = nk_path + path[k_path.len()..]
                             let mut new_p = nk_path;
                             new_p.extend_from_slice(&path[k_path.len()..]);
                             path = new_p;
                         }
                     }
                }
                
                if let Some(re) = &key_regex {
                    if re.is_match(&path.join(".")) {
                        key_match = true;
                        // Renaming with regex
                         let mut nk_path = args.new_key_part.clone();
                         if let Some(nk) = &args.new_key {
                             nk_path.extend(parse_key_path(nk));
                         }
                         if !nk_path.is_empty() {
                             path = nk_path;
                         }
                    }
                }

                // If key matched, check value conditions
                if key_match {
                    if let Some(ov) = &args.old_value {
                         if val_to_string(&val) == *ov {
                             if let Some(nv) = &new_val_parsed {
                                 val = nv.clone();
                             }
                         }
                    }
                    if let Some(re) = &old_value_regex {
                         if re.is_match(&val_to_string(&val)) {
                             if let Some(nv) = &new_val_parsed {
                                 val = nv.clone();
                             }
                         }
                    }
                    // Simple replacement
                    if let Some(v) = &args.value {
                         let v_parsed = parse_value(v, &args.type_) ?;
                         val = v_parsed;
                    }
                }
                
                new_entries.push((path, val));
            }
            
            let new_fm = unflatten_yaml(new_entries);
            write_result(file, Some(&new_fm), &content, &args.opts)?;
        }
    }
    Ok(())
}

pub fn run_init(args: InitArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    for file in files {
        let (fm, content) = read_front_matter(&file)?;
        if fm.is_none() {
            // Create empty FM
            write_result(&file, Some(&serde_yaml::Value::Mapping(serde_yaml::Mapping::new())), &content, &CommonOpts {
                dry_run: false, backup_suffix: None, backup_dir: None, stdout: false, output_dir: None
            })?;
        }
    }
    Ok(())
}

pub fn run_clean(args: CleanArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    for file in files {
        let (fm, content) = read_front_matter(&file)?;
        if let Some(f) = fm {
            if f.is_null() {
                 write_result(&file, None, &content, &CommonOpts {
                    dry_run: false, backup_suffix: None, backup_dir: None, stdout: false, output_dir: None
                })?;
            } else if let serde_yaml::Value::Mapping(m) = f {
                if m.is_empty() {
                     write_result(&file, None, &content, &CommonOpts {
                        dry_run: false, backup_suffix: None, backup_dir: None, stdout: false, output_dir: None
                    })?;
                }
            }
        }
    }
    Ok(())
}

pub fn run_validate(args: ValidateArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    for file in files {
        match read_front_matter(&file) {
            Ok(_) => println!("{}: OK", file.display()),
            Err(e) => println!("{}: Invalid ({})", file.display(), e),
        }
    }
    Ok(())
}

pub fn run_fmt(args: FmtArgs) -> Result<()> {
    let files = resolve_files(&args.files);
    for file in files {
        let (fm, content) = read_front_matter(&file)?;
        if let Some(f) = fm {
            write_result(&file, Some(&f), &content, &CommonOpts {
                dry_run: false, backup_suffix: None, backup_dir: None, stdout: false, output_dir: None
            })?;
        }
    }
    Ok(())
}
