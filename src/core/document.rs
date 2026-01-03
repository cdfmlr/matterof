use crate::core::selector::Selector;
use crate::core::path::{flatten_yaml, unflatten_yaml, insert_at_path};

pub struct Document {
    pub data: Option<serde_yaml::Value>,
    pub body: String,
}

impl Document {
    pub fn new(data: Option<serde_yaml::Value>, body: String) -> Self {
        Self { data, body }
    }

    pub fn select(&self, selector: &Selector) -> serde_yaml::Value {
        if let Some(fm) = &self.data {
            if selector.all { return fm.clone(); }
            
            let flattened = flatten_yaml(fm);
            let kept: Vec<_> = flattened.into_iter()
                .filter(|(path, val)| selector.matches(path, val))
                .collect();
            
            unflatten_yaml(kept)
        } else {
            serde_yaml::Value::Null
        }
    }

    pub fn set(&mut self, path: &[String], value: serde_yaml::Value) {
        let mut fm = self.data.take().unwrap_or_else(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        insert_at_path(&mut fm, path, value);
        self.data = Some(fm);
    }

    pub fn add(&mut self, path: &[String], value: serde_yaml::Value, index: Option<usize>) {
        if path.is_empty() { return; }
        let mut fm = self.data.take().unwrap_or_else(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        
        self.modify_at_path(&mut fm, path, |v| {
             if v.is_null() {
                 *v = serde_yaml::Value::Sequence(vec![value.clone()]);
             } else if let serde_yaml::Value::Sequence(seq) = v {
                 if let Some(idx) = index {
                     if idx <= seq.len() {
                         seq.insert(idx, value.clone());
                     } else {
                         seq.push(value.clone());
                     }
                 } else {
                     seq.push(value.clone());
                 }
             } else {
                 let old = v.clone();
                 *v = serde_yaml::Value::Sequence(vec![old, value.clone()]);
             }
        });
        
        self.data = Some(fm);
    }

    fn modify_at_path<F>(&self, root: &mut serde_yaml::Value, path: &[String], f: F) 
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

    pub fn remove(&mut self, selector: &Selector) {
        if let Some(fm) = self.data.take() {
            if selector.all {
                self.data = None;
                return;
            }
            let flattened = flatten_yaml(&fm);
            let kept: Vec<_> = flattened.into_iter()
                .filter(|(path, val)| !selector.matches(path, val))
                .collect();
            self.data = Some(unflatten_yaml(kept));
        }
    }

    pub fn replace(&mut self, selector: &Selector, new_val: Option<serde_yaml::Value>, new_key_path: Option<Vec<String>>) {
        if let Some(fm) = self.data.take() {
             let flattened = flatten_yaml(&fm);
             let mut result_entries = Vec::new();

             for (mut path, mut val) in flattened {
                 if selector.matches(&path, &val) {
                     if let Some(nv) = &new_val {
                         val = nv.clone();
                     }
                     if let Some(nk) = &new_key_path {
                         // Simple prefix replacement if applicable, or full path update
                         // This logic depends on CLI requirement. 
                         // For now, let's say full path replace if provided.
                         path = nk.clone();
                     }
                 }
                 result_entries.push((path, val));
             }
             self.data = Some(unflatten_yaml(result_entries));
        }
    }
}
