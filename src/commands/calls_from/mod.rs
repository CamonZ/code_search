mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

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

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for CallsFromCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
