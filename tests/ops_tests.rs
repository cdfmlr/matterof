use matterof::{
    core::{Document, Selector},
};

#[test]
fn test_document_select() {
    let fm: serde_yaml::Value = serde_yaml::from_str("title: Hello\ntags: [a, b]").unwrap();
    let doc = Document::new(Some(fm), "body".to_string());
    
    let mut selector = Selector::default();
    selector.keys = vec![vec!["title".to_string()]] ;
    
    let selected = doc.select(&selector);
    assert_eq!(selected["title"].as_str().unwrap(), "Hello");
    assert!(selected.get("tags").is_none());
}

#[test]
fn test_document_set() {
    let mut doc = Document::new(None, "body".to_string());
    doc.set(&["title".to_string()], serde_yaml::Value::from("New"));
    assert_eq!(doc.data.as_ref().unwrap()["title"].as_str().unwrap(), "New");
}

#[test]
fn test_document_add() {
    let fm: serde_yaml::Value = serde_yaml::from_str("tags: [a]").unwrap();
    let mut doc = Document::new(Some(fm), "body".to_string());
    doc.add(&["tags".to_string()], serde_yaml::Value::from("b"), None);
    let seq = doc.data.as_ref().unwrap()["tags"].as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[1].as_str().unwrap(), "b");
}

#[test]
fn test_document_remove() {
    let fm: serde_yaml::Value = serde_yaml::from_str("a: 1\nb: 2").unwrap();
    let mut doc = Document::new(Some(fm), "body".to_string());
    let mut selector = Selector::default();
    selector.keys = vec![vec!["a".to_string()]] ;
    doc.remove(&selector);
    assert!(doc.data.as_ref().unwrap().get("a").is_none());
    assert!(doc.data.as_ref().unwrap().get("b").is_some());
}