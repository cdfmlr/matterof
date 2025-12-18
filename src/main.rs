use clap::{ArgGroup, Parser};
use log::{debug, info};
use regex::Regex;
use matterof::{find_attachments, find_markdown_files_with_kv, print_files, rsync_files};
use std::path::PathBuf;
use walkdir::DirEntry;

// clap reference (example):
// - derive ArgGroup: https://github.com/clap-rs/clap/blob/v3.1.14/examples/tutorial_derive/README.md#argument-relations

/// smelt find markdown files with specific tag,
/// and rsync them to destination directory.
#[derive(Parser, Debug)]
#[command(name = "smelt", version = "0.1.1", author = "CDFMLR")]
#[clap(group(
    ArgGroup::new("action")
        .required(true)
        .args(&["print", "rsync_to"]),
))]
struct Cli {
    /// key of the tag to find in the markdown files' front matter.
    #[arg(short, long)]
    key: String,
    /// value of the key. A regex pattern indicating if the value matches.
    #[arg(short, long)]
    value: String,
    /// include attachment directories whose name matches the regex pattern,
    /// copy all files in them recursively. The attachment directories must
    /// be under the source directory.
    #[arg(short, long, value_name = "ATTACHMENT_DIR")]
    include_attachment: Option<String>,

    /// find files and print their paths
    #[arg(short, long, group = "action")]
    print: bool,
    /// find files and rsync to destination directory
    #[arg(short, long, value_name = "DEST", group = "action")]
    rsync_to: Option<PathBuf>, // 加 Option 才能 print 与 rsync_to 二选一

    /// source directory
    #[arg(value_name = "SRC")]
    src: PathBuf,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = &Cli::parse();
    debug!("cli opts: {:?}", cli);

    let value_re_str = &cli.value;
    let value_re = Regex::new(value_re_str)?;

    let files = find_markdown_files_with_kv(&cli.src, &cli.key, &value_re);
    info!(
        "built filter for markdown files with front matter [{}: {}]",
        cli.key, value_re_str
    );

    if cli.include_attachment.is_none() {
        return Action::from_cli(cli)?.execute(files);
    }

    let attachment_dir_re_str = cli.include_attachment.as_ref().unwrap();
    let attachment_dir_re = Regex::new(attachment_dir_re_str)?;

    let attachment_files = find_attachments(&cli.src, &attachment_dir_re);
    let files = files.chain(attachment_files);
    info!(
        "built filter for attachments with directory name matching [{}]",
        attachment_dir_re_str
    );

    Action::from_cli(cli)?.execute(files)
}

enum Action<'a> {
    Print,
    RsyncTo {
        src_dir: &'a PathBuf,
        dst_dir: &'a PathBuf,
    },
}

impl<'a> Action<'a> {
    fn from_cli(cli: &'a Cli) -> anyhow::Result<Self> {
        if cli.print {
            return Ok(Self::Print);
        }

        if let Some(dst_dir) = &cli.rsync_to {
            return Ok(Self::RsyncTo {
                src_dir: &cli.src,
                dst_dir: &dst_dir,
            });
        }

        return Err(anyhow::anyhow!("no action specified"));
    }

    fn execute(&self, files: impl Iterator<Item = DirEntry>) -> anyhow::Result<()> {
        return match self {
            Self::Print => {
                print_files(files);
                Ok(())
            }
            Self::RsyncTo { src_dir, dst_dir } => rsync_files(&src_dir, files, &dst_dir)
                .map_err(|e| anyhow::anyhow!("rsync error: {}", e)),
        };
    }
}
