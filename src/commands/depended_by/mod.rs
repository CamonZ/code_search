mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::Args;

/// Show what modules depend on a given module (incoming module dependencies)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search depended-by -m MyApp.Repo          # Who depends on Repo?
  code_search depended-by -m 'Ecto\\..*' -r      # Who depends on Ecto modules?")]
pub struct DependedByCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of dependents to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
