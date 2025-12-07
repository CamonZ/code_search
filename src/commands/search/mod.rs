mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::{Args, ValueEnum};

/// What to search for
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum SearchKind {
    /// Search for modules
    #[default]
    Modules,
    /// Search for functions
    Functions,
}

/// Search for modules or functions by name pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search search -p User                 # Find modules containing 'User'
  code_search search -p get_ -k functions    # Find functions starting with 'get_'
  code_search search -p '^MyApp\\.API' -r    # Regex match for module prefix")]
pub struct SearchCmd {
    /// Pattern to search for (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub pattern: String,

    /// What to search for
    #[arg(short, long, value_enum, default_value_t = SearchKind::Modules)]
    pub kind: SearchKind,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,

    /// Treat pattern as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,
}
