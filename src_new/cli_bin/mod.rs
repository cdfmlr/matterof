//! CLI module for the matterof command-line interface
//!
//! This module provides the command-line interface layer, including argument
//! parsing and command handlers that bridge CLI operations to library functions.

pub mod args;
pub mod commands;

// Re-exports are not needed since main.rs imports directly from submodules
