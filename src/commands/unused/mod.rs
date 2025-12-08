mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use clap::Args;

/// Find functions that are never called
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search unused                         # Find all unused functions
  code_search unused --public-only           # Find unused public API
  code_search unused -m MyApp.Accounts       # Filter to specific module
  code_search unused -Px                     # Public only, exclude generated
  code_search unused -m 'Accounts' --regex   # Match module with regex

  # Find orphan functions (private, never called internally):
  code_search unused --private-only

  # Find entry points (public functions not called internally):
  code_search unused --public-only -x        # Add -x to exclude __struct__ etc.")]
pub struct UnusedCmd {
    /// Module pattern to filter results (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module pattern as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Only show private functions (defp, defmacrop)
    #[arg(short, long, default_value_t = false, conflicts_with = "public_only")]
    pub private_only: bool,

    /// Only show public functions (def, defmacro)
    #[arg(short = 'P', long, default_value_t = false, conflicts_with = "private_only")]
    pub public_only: bool,

    /// Exclude compiler-generated functions (__struct__, __using__, __before_compile__, etc.)
    #[arg(short = 'x', long, default_value_t = false)]
    pub exclude_generated: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
