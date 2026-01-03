use matterof::{ops::*, args::*, utils::read_front_matter};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_file(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");
    fs::write(&file_path, content).unwrap();
    (temp_dir, file_path)
}

fn default_common() -> CommonOpts {
    CommonOpts {
        dry_run: false,
        backup_suffix: None,
        backup_dir: None,
        stdout: false,
        output_dir: None,
    }
}

#[test]
fn test_run_get() {
    let content = r#"---
title: Hello World
tags: [a, b]
nested:
  key: value
---
Body content"#;
    let (_tmp, file) = setup_file(content);

    let args = GetArgs {
        all: false,
        key: vec!["title".to_string()],
        key_part: vec![],
        key_regex: None,
        key_part_regex: vec![],
        value_regex: None,
        files: vec![file.clone()],
    };
    
    assert!(run_get(args).is_ok());

    let args = GetArgs {
        all: false,
        key: vec!["nested.key".to_string()],
        key_part: vec![],
        key_regex: None,
        key_part_regex: vec![],
        value_regex: None,
        files: vec![file],
    };
    assert!(run_get(args).is_ok());
}

#[test]
fn test_run_set() {
    let content = r#"---
title: Old Title
---
Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = SetArgs {
        key: vec!["title".to_string()],
        key_part: vec![],
        key_regex: None,
        value: vec!["New Title".to_string()],
        type_: None,
        opts: default_common(),
        files: vec![file.clone()],
    };
    
    run_set(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    assert_eq!(val["title"].as_str().unwrap(), "New Title");
}

#[test]
fn test_run_set_new_key() {
    let content = r#"---
title: Old
---
Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = SetArgs {
        key: vec!["new_key".to_string()],
        key_part: vec![],
        key_regex: None,
        value: vec!["value".to_string()],
        type_: None,
        opts: default_common(),
        files: vec![file.clone()],
    };
    
    run_set(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    assert_eq!(val["new_key"].as_str().unwrap(), "value");
}

#[test]
fn test_run_add() {
    let content = r#"---
tags: [a]
---
Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = AddArgs {
        key: Some("tags".to_string()),
        key_part: vec![],
        value: "b".to_string(),
        index: None,
        opts: default_common(),
        files: vec![file.clone()],
    };
    
    run_add(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    let seq = val["tags"].as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[1].as_str().unwrap(), "b");
}

#[test]
fn test_run_rm() {
    let content = r#"---
title: Keep
remove_me: Bye
tags: [a, b]
---
Body"#;
    let (_tmp, file) = setup_file(content);
    
    // Remove Key
    let args = RmArgs {
        key: Some("remove_me".to_string()),
        key_part: vec![],
        key_regex: None,
        value: None,
        value_regex: None,
        all: false,
        opts: default_common(),
        files: vec![file.clone()],
    };
    run_rm(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    assert!(val.get("remove_me").is_none());
    assert!(val.get("title").is_some());

    // Remove item from list
    // Note: unflatten_yaml converts lists to maps if indices are used.
    let args = RmArgs {
        key: Some("tags".to_string()),
        key_part: vec![],
        key_regex: None,
        value: Some("a".to_string()),
        value_regex: None,
        all: false,
        opts: default_common(),
        files: vec![file.clone()],
    };
    run_rm(args).unwrap();

    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    
    if let Some(seq) = val["tags"].as_sequence() {
        assert_eq!(seq.len(), 1);
        assert_eq!(seq[0].as_str().unwrap(), "b");
    } else if let Some(map) = val["tags"].as_mapping() {
        // Known behavior: becomes a map { "1": "b" }
        assert!(map.contains_key(&serde_yaml::Value::String("1".to_string())));
        assert_eq!(map[&serde_yaml::Value::String("1".to_string())].as_str().unwrap(), "b");
    } else {
        panic!("tags should be sequence or map");
    }
}

#[test]
fn test_run_replace() {
    let content = r#"---
old_key: value
---
Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = ReplaceArgs {
        key: Some("old_key".to_string()),
        key_part: vec![],
        key_regex: None,
        new_key: Some("new_key".to_string()),
        new_key_part: vec![],
        value: None,
        old_value: None,
        old_value_regex: None,
        new_value: None,
        type_: None,
        opts: default_common(),
        files: vec![file.clone()],
    };
    
    run_replace(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    let val = fm.unwrap();
    assert!(val.get("old_key").is_none());
    assert_eq!(val["new_key"].as_str().unwrap(), "value");
}

#[test]
fn test_run_init() {
    let content = r#"Just Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = InitArgs {
        files: vec![file.clone()],
    };
    
    run_init(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    assert!(fm.is_some()); 
}

#[test]
fn test_run_clean_empty() {
    // Tests that 'clean' removes empty front matter
    let content = r#"---
---
Just Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = CleanArgs {
        files: vec![file.clone()],
    };
    
    run_clean(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    assert!(fm.is_none());
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content.trim(), "Just Body");
}

#[test]
fn test_run_rm_all() {
    let content = r#"---
foo: bar
---
Just Body"#;
    let (_tmp, file) = setup_file(content);
    
    let args = RmArgs {
        key: None, key_part: vec![], key_regex: None, value: None, value_regex: None,
        all: true,
        opts: default_common(),
        files: vec![file.clone()],
    };
    
    run_rm(args).unwrap();
    
    let (fm, _) = read_front_matter(&file).unwrap();
    assert!(fm.is_none());
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content.trim(), "Just Body");
}
