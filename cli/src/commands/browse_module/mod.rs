use std::error::Error;

use clap::{Parser, ValueEnum};
use db::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};
use serde::Serialize;

mod cli_tests;
pub mod execute;
mod execute_tests;
pub mod output;
mod output_tests;

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

    #[command(flatten)]
    pub common: CommonArgs,
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

impl CommandRunner for BrowseModuleCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
