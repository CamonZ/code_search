//! Code analysis subcommands.

use clap::Subcommand;
use enum_dispatch::enum_dispatch;

use super::{
    AcceptsCmd, BoundariesCmd, BrowseModuleCmd, CallsFromCmd, CallsToCmd, ClustersCmd,
    ComplexityCmd, CyclesCmd, DependedByCmd, DependsOnCmd, DuplicatesCmd, FunctionCmd,
    GodModulesCmd, HotspotsCmd, ImportCmd, LargeFunctionsCmd, LocationCmd, ManyClausesCmd,
    PathCmd, ReturnsCmd, ReverseTraceCmd, SearchCmd, StructUsageCmd, TraceCmd, UnusedCmd,
};

#[derive(Subcommand, Debug)]
#[enum_dispatch(CommandRunner)]
pub enum CodeCommand {
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
    /// Analyze module connectivity using namespace-based clustering
    Clusters(ClustersCmd),
    /// Display complexity metrics for functions
    Complexity(ComplexityCmd),
    /// Detect circular dependencies between modules
    Cycles(CyclesCmd),
    /// Show function signature (args, return type)
    Function(FunctionCmd),
    /// Trace call chains from a starting function (forward traversal)
    Trace(TraceCmd),
    /// Trace call chains backwards - who calls the callers of a target
    ReverseTrace(ReverseTraceCmd),
    /// Find a call path between two functions
    Path(PathCmd),
    /// Find functions accepting a specific type pattern
    Accepts(AcceptsCmd),
    /// Find functions returning a specific type pattern
    Returns(ReturnsCmd),
    /// Find functions that accept or return a specific type pattern
    StructUsage(StructUsageCmd),
    /// Show what modules a given module depends on (outgoing module dependencies)
    DependsOn(DependsOnCmd),
    /// Show what modules depend on a given module (incoming module dependencies)
    DependedBy(DependedByCmd),
    /// Find functions that are never called
    Unused(UnusedCmd),
    /// Find functions with identical or near-identical implementations
    Duplicates(DuplicatesCmd),
    /// Find functions with the most incoming/outgoing calls
    Hotspots(HotspotsCmd),
    /// Find boundary modules - modules with high fan-in but low fan-out
    Boundaries(BoundariesCmd),
    /// Find god modules - modules with high function count and high connectivity
    GodModules(GodModulesCmd),
    /// Find large functions that may need refactoring
    LargeFunctions(LargeFunctionsCmd),
    /// Find functions with many pattern-matched heads
    ManyClauses(ManyClausesCmd),
}
