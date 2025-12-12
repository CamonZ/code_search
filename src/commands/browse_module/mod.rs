use clap::{Parser, ValueEnum};

pub mod execute;
pub mod output;

/// Browse definitions in a module or file
///
/// Unified command to explore all definitions (functions, specs, types, structs)
/// in a given module or file pattern. Returns all matching definitions grouped
/// and sorted by module and line number.
#[derive(Parser, Debug)]
pub struct BrowseModuleCmd {
    /// Module name, pattern, or file path to browse
    ///
    /// Can be:
    /// - Module name: "MyApp.Accounts" (exact match or pattern)
    /// - File path: "lib/accounts.ex" (substring or regex with --regex)
    /// - Pattern: "MyApp.*" (with --regex)
    pub module_or_file: String,

    /// Type of definitions to show
    ///
    /// If omitted, shows all definition types (functions, specs, types, structs).
    /// If specified, filters to only that type.
    #[arg(short, long)]
    pub kind: Option<DefinitionKind>,

    /// Filter by definition name (function/type/spec/struct name)
    ///
    /// Applies across all definition types. Supports substring match by default
    /// or regex match with --regex.
    #[arg(short, long)]
    pub name: Option<String>,

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

/// Type of definition to filter by
#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DefinitionKind {
    /// Function definitions
    Functions,
    /// @spec and @callback definitions
    Specs,
    /// @type, @typep, @opaque definitions
    Types,
    /// Struct definitions with fields
    Structs,
}

impl std::fmt::Display for DefinitionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefinitionKind::Functions => write!(f, "functions"),
            DefinitionKind::Specs => write!(f, "specs"),
            DefinitionKind::Types => write!(f, "types"),
            DefinitionKind::Structs => write!(f, "structs"),
        }
    }
}

use serde::Serialize;
