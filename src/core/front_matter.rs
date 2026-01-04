use crate::core::selector::{Resolver, get_value_at};
use crate::core::key_path::{KeyPath, ResolvedKeyPath, ResolvedKeyPathSegment};
use serde_yaml::Value;
use regex::Regex;

pub struct FrontMatter {
    pub value: Option<Value>,
}

pub struct QueryOptions {
    pub value_match: Option<String>,
    pub value_regex: Option<Regex>,
    pub all: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self { value_match: None, value_regex: None, all: false }
    }
}

impl FrontMatter {
    pub fn new(value: Option<Value>) -> Self {
        Self { value }
    }

    pub fn get(&self, key_path: &KeyPath, opts: &QueryOptions) -> Value {
        let root = match &self.value {
            Some(v) => v,
            None => return Value::Null,
        };

        let resolver = Resolver::new(root)
            .with_value_match(opts.value_match.clone())
            .with_value_regex(opts.value_regex.clone());

        let resolved_paths = if opts.all {
            resolver.resolve_all()
        } else {
            resolver.resolve(key_path)
        };

        let mut result = Value::Mapping(serde_yaml::Mapping::new());
        for rkp in resolved_paths {
            if let Some(val) = get_value_at(root, &rkp) {
                self.insert_at_resolved(&mut result, &rkp, val.clone());
            }
        }
        result
    }

    pub fn set(&mut self, key_path: &KeyPath, new_val: Value) {
        let mut root = self.value.take().unwrap_or_else(|| Value::Mapping(serde_yaml::Mapping::new()));
        let resolved = Resolver::new(&root).resolve(key_path);

        if resolved.is_empty() {
            if let Some(rkp) = key_path.to_resolved() {
                self.insert_at_resolved(&mut root, &rkp, new_val);
            }
        } else {
            for rkp in resolved {
                self.insert_at_resolved(&mut root, &rkp, new_val.clone());
            }
        }
        self.value = Some(root);
    }

    pub fn add(&mut self, key_path: &KeyPath, val: Value, index: Option<usize>) {
        let mut root = self.value.take().unwrap_or_else(|| Value::Mapping(serde_yaml::Mapping::new()));
        let resolved = Resolver::new(&root).resolve(key_path);

        let targets = if resolved.is_empty() {
             key_path.to_resolved().into_iter().collect()
        } else {
             resolved
        };

        for rkp in targets {
            self.modify_at_resolved(&mut root, &rkp, |target| {
                if target.is_null() {
                    *target = Value::Sequence(vec![val.clone()]);
                } else if let Value::Sequence(seq) = target {
                    let i = index.unwrap_or(seq.len());
                    if i <= seq.len() { seq.insert(i, val.clone()); }
                    else { seq.push(val.clone()); }
                } else {
                    let old = target.clone();
                    *target = Value::Sequence(vec![old, val.clone()]);
                }
            });
        }
        self.value = Some(root);
    }

    pub fn remove(&mut self, key_path: &KeyPath, opts: &QueryOptions) {
        let mut root = match self.value.take() {
            Some(r) => r,
            None => return,
        };

        if opts.all && opts.value_match.is_none() && opts.value_regex.is_none() {
            self.value = None;
            return;
        }

        let resolver = Resolver::new(&root)
            .with_value_match(opts.value_match.clone())
            .with_value_regex(opts.value_regex.clone());

        let resolved = if opts.all { resolver.resolve_all() } else { resolver.resolve(key_path) };
        
        for rkp in resolved {
            self.delete_at_resolved(&mut root, &rkp);
        }
        self.value = Some(root);
    }

    pub fn replace(&mut self, key_path: &KeyPath, new_val: Option<Value>, new_key_path: Option<KeyPath>, opts: &QueryOptions) {
        let mut root = match self.value.take() {
            Some(r) => r,
            None => return,
        };

        let resolver = Resolver::new(&root)
            .with_value_match(opts.value_match.clone())
            .with_value_regex(opts.value_regex.clone());

        let resolved = resolver.resolve(key_path);
        let mut changes = Vec::new();

        for rkp in resolved {
            let current_val = get_value_at(&root, &rkp).cloned().unwrap_or(Value::Null);
            self.delete_at_resolved(&mut root, &rkp);
            
            let target_val = new_val.clone().unwrap_or(current_val);
            let mut target_rkp = rkp;

            if let Some(kp) = &new_key_path {
                if let Some(resolved_new) = kp.to_resolved() {
                    target_rkp = resolved_new;
                }
            }
            changes.push((target_rkp, target_val));
        }

        for (p, v) in changes {
            self.insert_at_resolved(&mut root, &p, v);
        }
        self.value = Some(root);
    }

    fn insert_at_resolved(&self, root: &mut Value, path: &ResolvedKeyPath, val: Value) {
        if path.0.is_empty() { return; }
        let mut curr = root;
        for (i, seg) in path.0.iter().enumerate() {
            let is_last = i == path.0.len() - 1;
            match seg {
                ResolvedKeyPathSegment::Key(k) => {
                    if !curr.is_mapping() { *curr = Value::Mapping(serde_yaml::Mapping::new()); }
                    let map = curr.as_mapping_mut().unwrap();
                    let key = Value::String(k.clone());
                    if is_last {
                        map.insert(key, val.clone());
                        return;
                    } else {
                        if !map.contains_key(&key) || (!map.get(&key).unwrap().is_mapping() && !map.get(&key).unwrap().is_sequence()) {
                             map.insert(key.clone(), Value::Mapping(serde_yaml::Mapping::new()));
                        }
                        curr = map.get_mut(&key).unwrap();
                    }
                }
                ResolvedKeyPathSegment::Index(idx) => {
                    if !curr.is_sequence() { *curr = Value::Sequence(Vec::new()); }
                    let seq = curr.as_sequence_mut().unwrap();
                    while seq.len() <= *idx { seq.push(Value::Null); }
                    if is_last {
                        seq[*idx] = val.clone();
                        return;
                    } else {
                        if !seq[*idx].is_mapping() && !seq[*idx].is_sequence() {
                            seq[*idx] = Value::Mapping(serde_yaml::Mapping::new());
                        }
                        curr = &mut seq[*idx];
                    }
                }
            }
        }
    }

    fn delete_at_resolved(&self, root: &mut Value, path: &ResolvedKeyPath) {
        if path.0.is_empty() { return; }
        let mut curr = root;
        for (i, seg) in path.0.iter().enumerate() {
            let is_last = i == path.0.len() - 1;
            match seg {
                ResolvedKeyPathSegment::Key(k) => {
                    let key = Value::String(k.clone());
                    if let Some(map) = curr.as_mapping_mut() {
                        if is_last {
                            map.remove(&key);
                            return;
                        } else {
                            if let Some(next) = map.get_mut(&key) {
                                curr = next;
                            } else { return; }
                        }
                    } else { return; }
                }
                ResolvedKeyPathSegment::Index(idx) => {
                    if let Some(seq) = curr.as_sequence_mut() {
                        if is_last {
                            if *idx < seq.len() { seq.remove(*idx); }
                            return;
                        } else {
                            if let Some(next) = seq.get_mut(*idx) {
                                curr = next;
                            } else { return; }
                        }
                    } else { return; }
                }
            }
        }
    }

    fn modify_at_resolved<F>(&self, root: &mut Value, path: &ResolvedKeyPath, f: F) where F: FnOnce(&mut Value) {
        if path.0.is_empty() { return; }
        let mut curr = root;
        for seg in &path.0 {
            match seg {
                ResolvedKeyPathSegment::Key(k) => {
                    if !curr.is_mapping() { *curr = Value::Mapping(serde_yaml::Mapping::new()); }
                    let map = curr.as_mapping_mut().unwrap();
                    let key = Value::String(k.clone());
                    if !map.contains_key(&key) { map.insert(key.clone(), Value::Null); }
                    curr = map.get_mut(&key).unwrap();
                }
                ResolvedKeyPathSegment::Index(idx) => {
                    if !curr.is_sequence() { *curr = Value::Sequence(Vec::new()); }
                    let seq = curr.as_sequence_mut().unwrap();
                    while seq.len() <= *idx { seq.push(Value::Null); }
                    curr = &mut seq[*idx];
                }
            }
        }
        f(curr);
    }
}