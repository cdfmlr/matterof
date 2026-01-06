//! Main CLI entry point for the matterof tool
//!
//! This provides a clean separation between CLI parsing and library operations,
//! with proper error handling and logging setup.

use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};
use std::process;

// Import the CLI components directly since they're part of the binary
mod cli_bin;

use crate::cli_bin::args::{Cli, Commands};
use crate::cli_bin::commands::*;
use matterof::error::{MatterOfError, Result};

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup logging based on verbosity
    let log_level = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "error"
    } else {
        "info"
    };

    env_logger::Builder::from_env(Env::default().default_filter_or(log_level))
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    debug!(
        "Starting matterof with args: {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    // Execute the command and handle errors
    if let Err(error) = run_command(cli.command) {
        handle_error(error);
        process::exit(1);
    }

    debug!("Command completed successfully");
}

/// Run the appropriate command handler
fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Get(args) => {
            debug!("Running get command");
            get_command(args)
        }
        Commands::Set(args) => {
            debug!("Running set command");
            set_command(args)
        }
        Commands::Add(args) => {
            debug!("Running add command");
            add_command(args)
        }
        Commands::Remove(args) => {
            debug!("Running remove command");
            remove_command(args)
        }
        Commands::Replace(args) => {
            debug!("Running replace command");
            replace_command(args)
        }
        Commands::Query(args) => {
            debug!("Running query command");
            query_command(args)
        }
        Commands::Init(args) => {
            debug!("Running init command");
            init_command(args)
        }
        Commands::Clean(args) => {
            debug!("Running clean command");
            clean_command(args)
        }
        Commands::Validate(args) => {
            debug!("Running validate command");
            validate_command(args)
        }
        Commands::Format(args) => {
            debug!("Running format command");
            format_command(args)
        }
    }
}

/// Handle errors with appropriate logging and user-friendly messages
fn handle_error(error: MatterOfError) {
    match error {
        MatterOfError::FileNotFound { ref path } => {
            error!("File not found: {}", path.display());
        }
        MatterOfError::PermissionDenied { ref path } => {
            error!("Permission denied: {}", path.display());
        }
        MatterOfError::InvalidFrontMatter {
            ref path,
            ref reason,
        } => {
            error!("Invalid front matter in {}: {}", path.display(), reason);
        }

        MatterOfError::InvalidQuery { ref reason } => {
            error!("Invalid query: {}", reason);
        }
        MatterOfError::TypeConversion { ref from, ref to } => {
            error!("Cannot convert '{}' to {}", from, to);
        }
        MatterOfError::BackupError { ref reason } => {
            error!("Backup failed: {}", reason);
        }
        MatterOfError::Validation { ref message } => {
            error!("Validation error: {}", message);
        }
        MatterOfError::Multiple { ref errors } => {
            error!("Multiple errors occurred:");
            for (i, err) in errors.iter().enumerate() {
                error!("  {}: {}", i + 1, err);
            }
        }
        MatterOfError::Io(ref io_error) => {
            error!("I/O error: {}", io_error);
        }
        MatterOfError::Yaml(ref yaml_error) => {
            error!("YAML error: {}", yaml_error);
        }
        MatterOfError::Regex(ref regex_error) => {
            error!("Regular expression error: {}", regex_error);
        }
        _ => {
            error!("Error: {}", error);
        }
    }

    // Show additional context in debug mode
    debug!("Error details: {:?}", error);

    // Show suggestions for common errors
    match error {
        MatterOfError::FileNotFound { .. } => {
            info!("Tip: Make sure the file path is correct and the file exists");
        }

        MatterOfError::InvalidQuery { .. } => {
            info!("Tip: Check your regular expressions and query syntax");
        }
        MatterOfError::PermissionDenied { .. } => {
            info!("Tip: Make sure you have read/write permissions for the file");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    // Test module for main.rs

    #[test]
    fn test_main_compilation() {
        // This test just ensures the main module compiles correctly
        // More comprehensive tests would be in integration tests
    }
}
