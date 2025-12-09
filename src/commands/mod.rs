//! Command definitions and implementations.
//!
//! Each command is defined in its own module with:
//! - The command struct with clap attributes for CLI parsing

mod calls_from;
mod calls_to;
mod depended_by;
mod depends_on;
mod file;
mod function;
mod hotspots;
pub mod import;
mod location;
mod path;
mod reverse_trace;
mod search;
mod specs;
mod struct_cmd;
mod trace;
mod unused;

pub use calls_from::CallsFromCmd;
pub use calls_to::CallsToCmd;
pub use depended_by::DependedByCmd;
pub use depends_on::DependsOnCmd;
pub use file::FileCmd;
pub use function::FunctionCmd;
pub use hotspots::HotspotsCmd;
pub use import::ImportCmd;
pub use location::LocationCmd;
pub use path::PathCmd;
pub use reverse_trace::ReverseTraceCmd;
pub use search::SearchCmd;
pub use specs::SpecsCmd;
pub use struct_cmd::StructCmd;
pub use trace::TraceCmd;
pub use unused::UnusedCmd;

use clap::Subcommand;
use std::error::Error;

use cozo::DbInstance;

use crate::output::{OutputFormat, Outputable};

/// Trait for executing commands with command-specific result types.
pub trait Execute {
    type Output: Outputable;

    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>>;
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

    /// Show function signature (args, return type)
    Function(FunctionCmd),

    /// Show @spec and @callback definitions
    Specs(SpecsCmd),

    /// Show struct fields, defaults, and types
    Struct(StructCmd),

    /// Trace call chains from a starting function (forward traversal)
    Trace(TraceCmd),

    /// Trace call chains backwards - who calls the callers of a target
    ReverseTrace(ReverseTraceCmd),

    /// Find a call path between two functions
    Path(PathCmd),

    /// Show what modules a given module depends on (outgoing module dependencies)
    DependsOn(DependsOnCmd),

    /// Show what modules depend on a given module (incoming module dependencies)
    DependedBy(DependedByCmd),

    /// Find functions that are never called
    Unused(UnusedCmd),

    /// Find functions with the most incoming/outgoing calls
    Hotspots(HotspotsCmd),

    /// Show all functions defined in a file
    File(FileCmd),

    /// Catch-all for unknown commands
    #[command(external_subcommand)]
    Unknown(Vec<String>),
}

impl Command {
    /// Execute the command and return formatted output
    pub fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        match self {
            Command::Import(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Search(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Location(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::CallsFrom(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::CallsTo(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Function(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Specs(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Struct(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Trace(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::ReverseTrace(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Path(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::DependsOn(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::DependedBy(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Unused(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Hotspots(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::File(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Unknown(args) => {
                Err(format!("Unknown command: {}", args.first().unwrap_or(&String::new())).into())
            }
        }
    }
}
