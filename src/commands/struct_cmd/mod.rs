mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::Args;

/// Show struct fields, defaults, and types
///
/// Note: Named "struct_cmd" internally to avoid conflict with Rust's "struct" keyword
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search struct -m MyApp.User           # Show User struct definition
  code_search struct -m 'MyApp\\..*' -r      # All structs in MyApp namespace")]
pub struct StructCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
