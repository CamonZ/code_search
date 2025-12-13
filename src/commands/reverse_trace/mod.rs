mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
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

    /// Maximum depth to traverse (1-20)
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub depth: u32,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ReverseTraceCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
