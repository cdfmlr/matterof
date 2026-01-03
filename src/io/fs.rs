use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;
use anyhow::{Result, Context};

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

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read file {}", path.display()))
}

pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)
        .with_context(|| format!("failed to write file {}", path.display()))
}
