//! Command definitions and implementations.
//!
//! Each command is defined in its own module with:
//! - The command struct with clap attributes for CLI parsing
//! - Common arguments shared via [`CommonArgs`]

use clap::Args;

/// Common arguments shared across most commands.
///
/// Use `#[command(flatten)]` to include these in a command struct:
/// ```ignore
/// pub struct MyCmd {
///     pub module: String,
///     #[command(flatten)]
///     pub common: CommonArgs,
/// }
/// ```
#[derive(Args, Debug, Clone)]
pub struct CommonArgs {
    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat patterns as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

mod boundaries;
mod browse_module;
mod calls_from;
mod calls_to;
mod depended_by;
mod depends_on;
mod function;
mod god_modules;
mod hotspots;
pub mod import;
mod location;
mod path;
mod reverse_trace;
mod search;
mod trace;
mod unused;

pub use boundaries::BoundariesCmd;
pub use browse_module::BrowseModuleCmd;
pub use calls_from::CallsFromCmd;
pub use calls_to::CallsToCmd;
pub use depended_by::DependedByCmd;
pub use depends_on::DependsOnCmd;
pub use function::FunctionCmd;
pub use god_modules::GodModulesCmd;
pub use hotspots::HotspotsCmd;
pub use import::ImportCmd;
pub use location::LocationCmd;
pub use path::PathCmd;
pub use reverse_trace::ReverseTraceCmd;
pub use search::SearchCmd;
pub use trace::TraceCmd;
pub use unused::UnusedCmd;

use clap::Subcommand;
use enum_dispatch::enum_dispatch;
use std::error::Error;

use cozo::DbInstance;

use crate::output::{OutputFormat, Outputable};

/// Trait for executing commands with command-specific result types.
pub trait Execute {
    type Output: Outputable;

    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>>;
}

/// Trait for commands that can be executed and formatted.
/// Auto-implemented for all Command variants via enum_dispatch.
#[enum_dispatch]
pub trait CommandRunner {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>>;
}

#[derive(Subcommand, Debug)]
#[enum_dispatch(CommandRunner)]
pub enum Command {
    /// Import a call graph JSON file into the database
    Import(ImportCmd),

    /// Browse all definitions in a module or file
    BrowseModule(BrowseModuleCmd),

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

    /// Find boundary modules - modules with high fan-in but low fan-out
    Boundaries(BoundariesCmd),

    /// Find god modules - modules with high function count and high connectivity
    GodModules(GodModulesCmd),

    /// Catch-all for unknown commands
    #[command(external_subcommand)]
    Unknown(Vec<String>),
}

// CommandRunner implementations are provided by each command's module.
// The enum_dispatch macro automatically generates dispatch logic for the Command enum.

// Special handling for Unknown variant - not a real command
impl CommandRunner for Vec<String> {
    fn run(self, _db: &DbInstance, _format: OutputFormat) -> Result<String, Box<dyn Error>> {
        Err(format!("Unknown command: {}", self.first().unwrap_or(&String::new())).into())
    }
}
