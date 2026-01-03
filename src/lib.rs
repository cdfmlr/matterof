use anyhow::{Context, Result};
use gray_matter::Matter;
use gray_matter::engine::YAML;
use log::{debug, info, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// is_markdown checks if the given DirEntry is a markdown file.
fn is_markdown(entry: &DirEntry) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }

    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".md"))
        .unwrap_or(false)
}

/// MATTER is a singleton that can be used to parse
/// markdown files, and extract the YAML front matters.
static MATTER: Lazy<Matter<YAML>> = Lazy::new(|| Matter::<YAML>::new());

#[derive(Debug, Error)]
pub enum CheckMarkdownFrontMatterError {
    #[error("failed to walk dir: {0}")]
    WalkDirIterError(#[from] walkdir::Error),

    #[error("failed to read {path}: {source}")]
    ReadFileError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse front matter as a map: {0}")]
    AsHashmapError(#[source] gray_matter::Error),

    #[error("front matter missing")]
    RefDataNone,

    #[error("failed to extract key `{key}` as string: {source}")]
    AsStringError {
        key: String,
        #[source]
        source: gray_matter::Error,
    },
}

use CheckMarkdownFrontMatterError::*;

/// contains_kv checks if the given markdown file contains
/// the given key/value pair in its YAML front matter.
fn contains_kv(markdown_file: &Path, key: &str, value: &Regex) -> Result<bool> {
    let content = fs::read_to_string(markdown_file)
        .map_err(|source| ReadFileError {
            path: markdown_file.to_path_buf(),
            source,
        })
        .with_context(|| format!("reading {}", markdown_file.display()))?;

    let parsed = MATTER.parse(content.trim());
    let Some(data) = parsed.data.as_ref() else {
        return Ok(false);
    };
    let map = data
        .as_hashmap()
        .map_err(AsHashmapError)
        .with_context(|| format!("parsing front matter in {}", markdown_file.display()))?;

    let Some(raw) = map.get(key) else {
        return Ok(false);
    };
    let Ok(got_value) = raw.as_string() else {
        warn!(
            "skipping {}: key `{}` is not a string",
            markdown_file.display(),
            key
        );
        return Ok(false);
    };

    Ok(value.is_match(&got_value))
}

/// find_markdown_files walks the given directory and returns
/// an iter of all markdown files.
fn find_markdown_files(dir: &Path) -> impl Iterator<Item = DirEntry> + use<> {
    walkdir_iter(dir).filter(is_markdown)
}

/// find_markdown_files_with_kv walks the given directory and returns
/// an iter of all markdown files that contain the given key/value pair
/// in their YAML front matter.
pub fn find_markdown_files_with_kv<'a>(
    dir: &'a Path,
    key: &'a str,
    value: &'a Regex,
) -> impl Iterator<Item = Result<DirEntry>> + 'a {
    find_markdown_files(dir).filter_map(move |entry| {
        let path = entry.path().to_path_buf();
        contains_kv(&path, key, value)
            .map(|matched| matched.then_some(entry))
            .with_context(|| format!("checking {}", path.display()))
            .transpose()
    })
}

/// print_files prints the path of each file in the given iter.
pub fn print_files(files: impl Iterator<Item = Result<DirEntry>>) -> Result<()> {
    for file in files {
        println!("{}", file?.path().display());
    }
    Ok(())
}

// #[derive(Error, Debug)]
// pub enum RsyncFilesError {
//     #[error("failed to create temp dir: {0}")]
//     CreateTempDirError(io::Error),
//     #[error("failed to create sub dir in temp dir: {reason}")]
//     SubDirInTempDirError{reason: String},
//     #[error("failed to hard link {src} to {dst}: {err}")]
//     HardLinkError{src: String, dst: String, err: io::Error},
// }
//
// use RsyncFilesError::*;

/// rsync_files hard links all files in the given iter to a temporary directory
/// and then exec rsync(1) to sync the temporary directory to the given dst.
///
/// Errors if any of the src_files is not in the src_base_dir.
///
/// The temporary directory is necessary here to
/// - make a "view" of the filtered src files (src_files);
/// - keep them in the same directory structure as the src_base_dir;
/// - make sure only src_files are synced to the dst.
pub fn rsync_files(
    src_base_dir: &Path,
    src_files: impl Iterator<Item = Result<DirEntry>>,
    dst: &Path,
) -> anyhow::Result<()> {
    info!(
        "rsync filtered files from {} to {}",
        src_base_dir.display(),
        dst.display()
    );

    let tmp_dir = tempfile::tempdir()?;
    let tmp_dir = tmp_dir.path().to_owned();
    // to_owned() to drop tmp_dir after this function
    // drop(TempDir) do rm -rf tmp_dir by std::fs::remove_dir_all

    info!(
        "created temp dir {} to store filtered files",
        tmp_dir.display()
    );

    let mut cnt = 0;
    for file in src_files {
        let file = file?;
        let s = file.path();
        let d = &tmp_dir.join(s.strip_prefix(src_base_dir)?);
        let d_parent_dir = d.parent().unwrap();

        if !d_parent_dir.exists() {
            fs::create_dir_all(d_parent_dir)?;
        }

        if d.exists() {
            warn!("file {:?} already exists, skip", d);
            continue;
        }

        fs::hard_link(s, d)?;
        debug!("hard linked {:?} to {:?}", s, d);
        cnt += 1;
    }
    info!(
        "hard linked {} files from {} to {}",
        cnt,
        src_base_dir.display(),
        tmp_dir.display()
    );

    // add a trailing slash to src:
    //   rsync /path/to/src/ /path/to/dst
    // to make sure /path/to/src/{file} is synced to /path/to/dst/{file}
    // instead of /path/to/dst/src/{file}
    let rsync_src_dir = &dir_path_with_tail_slash(&tmp_dir);

    info!(
        "exec: rsync -av --delete {} {}",
        rsync_src_dir,
        dst.display()
    );

    let status = std::process::Command::new("rsync")
        .arg("-av")
        // .arg("--delete")
        .arg(rsync_src_dir)
        .arg(dst)
        .status()?;

    assert!(status.success());

    Ok(())
}

/// path_to_string safely converts a Path to String
fn path_to_string(path: &Path) -> String {
    path.to_str().unwrap_or("").to_owned()
}

/// dir_path_with_tail_slash converts a Path to a String,
/// and append a tailing slash to the String if it doesn't have one.
///
/// ```
/// // assert!(dir_path_with_tail_slash(Path::new("path/to/hello")) == "path/to/hello/");
/// ```
fn dir_path_with_tail_slash(dir: &Path) -> String {
    let mut dir = path_to_string(dir);
    let slash = if cfg!(windows) { "\\" } else { "/" };

    if !dir.ends_with(slash) {
        dir.push_str(slash);
    }

    dir
}

/// walkdir_iter is a wrapper of walkdir::WalkDir::new(dir).into_iter().
/// It filters out any error, log and ignore it.
fn walkdir_iter(dir: &Path) -> impl Iterator<Item = DirEntry> + use<> {
    WalkDir::new(dir).into_iter().filter_map(|e| match e {
        Ok(e) => Some(e),
        Err(e) => {
            let e = WalkDirIterError(e);
            warn!("failed to walk dir: {:?}", e);
            None
        }
    })
}

pub fn find_attachments<'a>(
    dir: &'a Path,
    attachment_dir_re: &'a Regex,
) -> impl Iterator<Item = DirEntry> + 'a {
    walkdir_iter(dir).filter(|e| e.file_type().is_file() && is_attachment(attachment_dir_re, e))
}

/// is_attachment check all ancestors of the file
/// if any ancestor matches the regex, trait it as an attachment
fn is_attachment(attachment_dir_re: &Regex, file: &DirEntry) -> bool {
    for ancestor in file.path().ancestors() {
        let ancestor = ancestor.to_str().unwrap();
        if attachment_dir_re.is_match(ancestor) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::debug;
    use std::env;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() -> () {
        INIT.call_once(|| {
            if let Err(_) = env::var("RUST_LOG") {
                // TODO: Audit that the environment access only happens in single-threaded code.
                unsafe { env::set_var("RUST_LOG", "debug") };
            }

            let _ = env_logger::builder().is_test(true).try_init();
            debug!(
                "tests: use env_logger with RUST_LOG={}",
                env::var("RUST_LOG").unwrap_or("".to_string())
            );
        });
    }

    #[test]
    fn test_is_markdown() {
        setup();

        let dir = Path::new("test_resc");

        let markdowns = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(is_markdown);

        let markdowns: Vec<_> = markdowns.collect();

        assert_eq!(markdowns.len(), 5);

        assert!(
            markdowns
                .iter()
                .all(|e| e.file_name().to_str().unwrap().ends_with(".md"))
        );

        let file_names = markdowns
            .iter()
            .map(|e| e.file_name().to_str().unwrap())
            .collect::<Vec<_>>();

        assert!(file_names.contains(&"has_yaml.md"));
        assert!(file_names.contains(&"missing_yaml.md"));
        assert!(file_names.contains(&"missing_key.md"));
        assert!(file_names.contains(&"bad_value_type.md"));
        assert!(file_names.contains(&"atta.md"));
    }

    #[test]
    fn test_contains_tag() {
        setup();

        let dir = Path::new("test_resc");
        let file_with_yaml = dir.join("has_yaml.md");
        let file_wo_yaml = dir.join("missing_yaml.md");
        let file_not_exist = dir.join("not_exist.md");

        assert!(
            contains_kv(
                &file_with_yaml,
                "publish_to",
                &Regex::new("hello-world").unwrap()
            )
            .unwrap()
        );
        assert!(
            !contains_kv(
                &file_wo_yaml,
                "publish_to",
                &Regex::new("hello-world").unwrap()
            )
            .unwrap()
        );

        assert!(contains_kv(&file_not_exist, "tags", &Regex::new("rust").unwrap()).is_err());
    }

    #[test]
    fn test_find_markdown_files() {
        setup();

        let dir = Path::new("test_resc");
        let files: Vec<_> = find_markdown_files(&dir).collect();

        assert_eq!(files.len(), 5);
        assert!(files.iter().any(|e| e.path().ends_with("has_yaml.md")));
        assert!(files.iter().any(|e| e.path().ends_with("missing_yaml.md")));
        assert!(
            files
                .iter()
                .any(|e| e.path().ends_with("sub_dir/missing_key.md"))
        );
        assert!(
            files
                .iter()
                .any(|e| e.path().ends_with("sub_dir/bad_value_type.md"))
        );
        assert!(files.iter().any(|e| e.path().ends_with("atta.md")));
    }

    #[test]
    fn test_find_markdown_files_with_tag() {
        setup();

        let dir = Path::new("test_resc");
        let files: Vec<_> =
            find_markdown_files_with_kv(&dir, "publish_to", &Regex::new("hello-world").unwrap())
                .collect::<Result<Vec<_>>>()
                .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.iter().any(|e| e.path().ends_with("has_yaml.md")));
    }

    #[test]
    fn test_rsync_files() {
        setup();

        let src_dir = Path::new("test_resc");
        let dst_dir = tempfile::tempdir().unwrap();
        let dst_dir = dst_dir.path().to_owned();

        debug!("dst_dir: {:?}", dst_dir);

        let value_re = Regex::new("expect copy").unwrap();
        let files = find_markdown_files_with_kv(&src_dir, "rsync_test", &value_re);

        let result = rsync_files(&src_dir, files, &dst_dir);
        assert!(result.is_ok());

        let mut dst_files: Vec<_> = find_markdown_files(&dst_dir).collect();

        debug!("dst_files: {:?}", dst_files);

        assert_eq!(dst_files.len(), 2);

        // check the full path of each file

        dst_files.sort_by(|a, b| a.path().cmp(&b.path()));
        let dst_files_path = dst_files
            .iter()
            .map(|e| e.path().to_owned())
            .collect::<Vec<_>>();

        let mut expected_path = vec![
            dst_dir.join("has_yaml.md"),
            dst_dir.join("sub_dir/bad_value_type.md"),
        ];

        expected_path.sort_by(|a, b| a.as_path().cmp(&b.as_path()));

        assert_eq!(dst_files_path, expected_path);
    }

    #[test]
    fn test_find_attachments() {
        setup();

        let dir = Path::new("test_resc");
        let attachment_dir_re = Regex::new(r"attachment").unwrap();
        let files: Vec<_> = find_attachments(&dir, &attachment_dir_re).collect();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|e| {
            e.path()
                .ends_with("test_resc/attachment/atta_sub_dir/atta.md")
        }));
        assert!(
            files
                .iter()
                .any(|e| e.path().ends_with("test_resc/attachment/atta_root.txt"))
        );
        assert!(files.iter().any(|e| {
            e.path()
                .ends_with("test_resc/sub_dir/attachment/atta_sub.txt")
        }));
    }
}
