//! Command definitions and implementations.
//!
//! Each command is defined in its own module with:
//! - The command struct with clap attributes for CLI parsing

mod calls_from;
mod calls_to;
mod import;
mod location;
mod search;

pub use calls_from::CallsFromCmd;
pub use calls_to::CallsToCmd;
pub use import::ImportCmd;
pub use location::LocationCmd;
pub use search::SearchCmd;

use clap::Subcommand;
use std::error::Error;
use std::path::Path;

use crate::output::{OutputFormat, Outputable};

/// Trait for executing commands with command-specific result types.
pub trait Execute {
    type Output: Outputable;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>>;
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Import a call graph JSON file into the database
    Import(ImportCmd),

    /// Search for modules or functions by name pattern
    Search(SearchCmd),

    /// Find where a function is defined (file:line_start:line_end)
    Location(LocationCmd),

    /// Show what a module/function calls (outgoing edges)
    CallsFrom(CallsFromCmd),

    /// Show what calls a module/function (incoming edges)
    CallsTo(CallsToCmd),

    /// Catch-all for unknown commands
    #[command(external_subcommand)]
    Unknown(Vec<String>),
}

impl Command {
    /// Execute the command and return formatted output
    pub fn run(self, db_path: &Path, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        match self {
            Command::Import(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::Search(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::Location(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::CallsFrom(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::CallsTo(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::Unknown(args) => {
                Err(format!("Unknown command: {}", args.first().unwrap_or(&String::new())).into())
            }
        }
    }
}
