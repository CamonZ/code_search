mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use clap::Args;

/// Show @spec and @callback definitions for a module
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search specs MyApp.Module             # All specs in module
  code_search specs MyApp -f get_user        # Specs for specific function
  code_search specs MyApp -k callback        # Only callbacks
  code_search specs 'MyApp.*' -r             # Regex pattern matching")]
pub struct SpecsCmd {
    /// Module name (exact match or pattern with --regex)
    pub module: String,

    /// Filter by function name
    #[arg(short = 'f', long)]
    pub function: Option<String>,

    /// Filter by kind (spec or callback)
    #[arg(short, long)]
    pub kind: Option<String>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module and function as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
