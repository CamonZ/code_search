mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::Args;

/// Show what a module/function calls (outgoing edges)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search calls-from -m MyApp.Accounts           # All calls from module
  code_search calls-from -m MyApp -f get_user        # Calls from specific function
  code_search calls-from -m MyApp -f get_user -a 1   # With specific arity")]
pub struct CallsFromCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Function name (optional, if not specified shows all calls from module)
    #[arg(short = 'f', long)]
    pub function: Option<String>,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

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
