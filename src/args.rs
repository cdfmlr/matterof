use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    /// Get front-matter key-values
    Get(GetArgs),
    /// Set front-matter key-values
    Set(SetArgs),
    /// Add values to a list/mapping
    Add(AddArgs),
    /// Remove keys/values
    Rm(RmArgs),
    /// Replace keys/values
    Replace(ReplaceArgs),
    /// Initialize front-matter if not exists
    Init(InitArgs),
    /// Remove front-matter if empty
    Clean(CleanArgs),
    /// Validate front-matter syntax
    Validate(ValidateArgs),
    /// Format front-matter
    Fmt(FmtArgs),
}

#[derive(Args, Debug)]
pub struct CommonOpts {
    /// Preview changes without modifying files
    #[arg(long)]
    pub dry_run: bool,
    /// Create a backup copy with suffix
    #[arg(long)]
    pub backup_suffix: Option<String>,
    /// Create a backup copy to a specific directory
    #[arg(long)]
    pub backup_dir: Option<PathBuf>,
    /// Output modified content to stdout
    #[arg(long)]
    pub stdout: bool,
    /// Output modified files to a specific directory
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Get all key-values
    #[arg(long)]
    pub all: bool,
    /// Key to get
    #[arg(long)]
    pub key: Vec<String>,
    /// Key parts for nested keys
    #[arg(long = "key-part")]
    pub key_part: Vec<String>,
    /// Regex to match keys
    #[arg(long = "key-regex")]
    pub key_regex: Option<String>,
    /// Regex to match key parts
    #[arg(long = "key-part-regex")]
    pub key_part_regex: Vec<String>,
    /// Regex to match values
    #[arg(long = "value-regex")]
    pub value_regex: Option<String>,

    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    #[arg(long)]
    pub key: Vec<String>,
    #[arg(long = "key-part")]
    pub key_part: Vec<String>,
    #[arg(long = "key-regex")]
    pub key_regex: Option<String>,

    #[arg(long)]
    pub value: Vec<String>,
    
    /// Value type: string, int, float, bool
    #[arg(long = "type")]
    pub type_: Option<String>,

    #[command(flatten)]
    pub opts: CommonOpts,

    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long = "key-part")]
    pub key_part: Vec<String>,

    #[arg(long)]
    pub value: String,
    
    /// Insert at specific index
    #[arg(long)]
    pub index: Option<usize>,

    #[command(flatten)]
    pub opts: CommonOpts,

    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RmArgs {
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long = "key-part")]
    pub key_part: Vec<String>,
    #[arg(long = "key-regex")]
    pub key_regex: Option<String>,

    #[arg(long)]
    pub value: Option<String>,
    #[arg(long = "value-regex")]
    pub value_regex: Option<String>,

    #[arg(long)]
    pub all: bool,

    #[command(flatten)]
    pub opts: CommonOpts,

    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ReplaceArgs {
    #[arg(long)]
    pub key: Option<String>,
    #[arg(long = "key-part")]
    pub key_part: Vec<String>,
    #[arg(long = "key-regex")]
    pub key_regex: Option<String>,

    #[arg(long)]
    pub new_key: Option<String>,
    #[arg(long = "new-key-part")]
    pub new_key_part: Vec<String>,

    #[arg(long)]
    pub value: Option<String>, // In case alias of set
    
    #[arg(long)]
    pub old_value: Option<String>,
    #[arg(long = "old-value-regex")]
    pub old_value_regex: Option<String>,
    
    #[arg(long)]
    pub new_value: Option<String>,

    #[arg(long = "type")]
    pub type_: Option<String>,

    #[command(flatten)]
    pub opts: CommonOpts,

    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct InitArgs {
    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct CleanArgs {
    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct FmtArgs {
    pub files: Vec<PathBuf>,
}
