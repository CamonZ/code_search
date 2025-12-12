mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, Execute};
use crate::output::{OutputFormat, Outputable};

/// Trace call chains backwards - who calls the callers of a target
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search reverse-trace -m MyApp.Repo -f get     # Who ultimately calls Repo.get?
  code_search reverse-trace -m Ecto.Repo -f insert --depth 10  # Deeper traversal
  code_search reverse-trace -m MyApp -f 'handle_.*' -r   # Regex pattern")]
pub struct ReverseTraceCmd {
    /// Target module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Target function name (exact match or pattern with --regex)
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

impl CommandRunner for ReverseTraceCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
