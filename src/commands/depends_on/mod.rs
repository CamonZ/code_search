mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::Args;

/// Show what modules a given module depends on (outgoing module dependencies)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search depends-on -m MyApp.Accounts       # What does Accounts depend on?
  code_search depends-on -m 'MyApp\\.Web.*' -r   # Dependencies of Web modules")]
pub struct DependsOnCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of dependencies to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
