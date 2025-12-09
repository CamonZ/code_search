mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use clap::Args;

/// Show @type, @typep, and @opaque definitions for a module
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search types MyApp.Module             # All types in module
  code_search types MyApp -n user            # Types matching 'user'
  code_search types MyApp -k opaque          # Only opaque types
  code_search types 'MyApp.*' -r             # Regex pattern matching")]
pub struct TypesCmd {
    /// Module name (exact match or pattern with --regex)
    pub module: String,

    /// Filter by type name
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Filter by kind (type, typep, opaque)
    #[arg(short, long)]
    pub kind: Option<String>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module and name as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
