//! Command definitions and implementations.
//!
//! Each command is defined in its own module with:
//! - The command struct with clap attributes for CLI parsing

mod import;

pub use import::ImportCmd;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Import a call graph JSON file into the database
    Import(ImportCmd),
}
