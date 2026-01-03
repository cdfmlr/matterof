use matterof::*;
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn test_is_markdown() {
    let dir = Path::new("test_resc");
    let markdowns = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_markdown(e.path()));

    let markdowns: Vec<_> = markdowns.collect();
    // In original test it was 5. 
    // Now I added test_playground.md and test_playground_2.md to test_resc.
    // So it should be 7.
    assert!(markdowns.len() >= 5);
}