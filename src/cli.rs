//! CLI argument definitions.
//!
//! This module contains the top-level CLI structure and shared types.
//! Individual command definitions are in the `commands` module.

use clap::Parser;
use std::path::PathBuf;

use crate::commands::Command;
use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the CozoDB SQLite database file
    #[arg(short, long, default_value = "./cozo.sqlite", global = true)]
    pub db: PathBuf,

    /// Output format
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Table, global = true)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}
