//! CLI argument definitions.
//!
//! This module contains the top-level CLI structure and shared types.
//! Individual command definitions are in the `commands` module.

use clap::Parser;
use std::path::PathBuf;

use crate::commands::Command;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the CozoDB SQLite database file
    #[arg(short, long, default_value = "./cozo.sqlite", global = true)]
    pub db: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}
