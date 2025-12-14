//! CLI argument definitions.
//!
//! This module contains the top-level CLI structure and shared types.
//! Individual command definitions are in the `commands` module.

use clap::Parser;

use crate::commands::Command;
use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Database connection (path or URL).
    ///
    /// Examples:
    ///   ./cozo.sqlite           (CozoDB/SQLite file)
    ///   :memory:                (CozoDB in-memory)
    ///   sqlite:///path/to/db    (explicit SQLite URL)
    ///   postgres://user@host/db (PostgreSQL - future)
    #[arg(short, long, default_value = "./cozo.sqlite", global = true)]
    pub db: String,

    /// Output format
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Table, global = true)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}
