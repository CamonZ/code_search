mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use db::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Trace call chains from a starting function (forward traversal)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search trace MyApp.Web index                  # Trace from controller action
  code_search trace MyApp handle_call --depth 10    # Deeper traversal
  code_search trace -r 'MyApp\\..*' 'handle_.*'      # Regex pattern
")]
pub struct TraceCmd {
    /// Starting module name (exact match or pattern with --regex)
    pub module: String,

    /// Starting function name (exact match or pattern with --regex)
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

impl CommandRunner for TraceCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
