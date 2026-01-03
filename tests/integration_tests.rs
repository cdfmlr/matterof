use matterof::*;
use std::fs;
use tempfile::TempDir;
use walkdir::WalkDir;

#[test]
fn test_is_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::write(root.join("a.md"), "").unwrap();
    fs::write(root.join("b.markdown"), "").unwrap();
    fs::write(root.join("c.txt"), "").unwrap();
    fs::create_dir(root.join("subdir")).unwrap();
    fs::write(root.join("subdir/d.md"), "").unwrap();

    let markdowns: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_markdown(e.path()))
        .collect();

    assert_eq!(markdowns.len(), 3);
}
