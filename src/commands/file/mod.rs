mod cli_tests;
mod execute;
mod output;

use clap::Args;

/// Show all functions defined in a file
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search file -f lib/accounts.ex        # Functions in specific file
  code_search file -f accounts               # Files containing 'accounts'
  code_search file -f 'lib/.*_test.ex' -r    # All test files with regex")]
pub struct FileCmd {
    /// File path pattern (substring match by default, regex with --regex)
    #[arg(short = 'f', long)]
    pub file: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat file path as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
