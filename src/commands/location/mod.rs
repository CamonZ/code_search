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

/// Find where a function is defined (file:line_start:line_end)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search location -f get_user           # Find all get_user functions
  code_search location -m MyApp -f get_user  # In specific module
  code_search location -f get_user -a 1      # With specific arity
  code_search location -f 'get_.*' -r        # Regex pattern matching")]
pub struct LocationCmd {
    /// Module name (exact match or pattern with --regex). If not specified, searches all modules.
    #[arg(short, long)]
    pub module: Option<String>,

    /// Function name (exact match or pattern with --regex)
    #[arg(short = 'f', long)]
    pub function: String,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for LocationCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
