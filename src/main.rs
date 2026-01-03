use anyhow::Result;
use clap::Parser;
use matterof::{Commands, run_get, run_set, run_add, run_rm, run_replace, run_init, run_clean, run_validate, run_fmt};

#[derive(Parser)]
#[command(name = "matterof", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Get(args) => run_get(args),
        Commands::Set(args) => run_set(args),
        Commands::Add(args) => run_add(args),
        Commands::Rm(args) => run_rm(args),
        Commands::Replace(args) => run_replace(args),
        Commands::Init(args) => run_init(args),
        Commands::Clean(args) => run_clean(args),
        Commands::Validate(args) => run_validate(args),
        Commands::Fmt(args) => run_fmt(args),
    }
}
