mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Show function signature (args, return type)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search function MyApp.Accounts get_user       # Show signature
  code_search function MyApp.Accounts get_user -a 1  # Specific arity
  code_search function -r 'MyApp\\..*' 'get_.*'      # Regex matching
")]
pub struct FunctionCmd {
    /// Module name (exact match or pattern with --regex)
    pub module: String,

    /// Function name (exact match or pattern with --regex)
    pub function: String,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for FunctionCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
