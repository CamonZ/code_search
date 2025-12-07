mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use clap::Args;

/// Trace call chains from a starting function (forward traversal)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search trace -m MyApp.Web -f index            # Trace from controller action
  code_search trace -m MyApp -f handle_call --depth 10   # Deeper traversal
  code_search trace -m 'MyApp\\..*' -f 'handle_.*' -r    # Regex pattern")]
pub struct TraceCmd {
    /// Starting module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Starting function name (exact match or pattern with --regex)
    #[arg(short = 'f', long)]
    pub function: String,

    /// Function arity (optional)
    #[arg(short, long)]
    pub arity: Option<i64>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module and function as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum depth to traverse (1-20)
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub depth: u32,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
