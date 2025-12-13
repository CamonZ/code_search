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

/// Show what calls a module/function (incoming edges)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search calls-to -m MyApp.Repo                 # All callers of module
  code_search calls-to -m MyApp.Repo -f get          # Callers of specific function
  code_search calls-to -m MyApp.Repo -f get -a 2     # With specific arity

  # Find all call sites (replaces 'call-sites' command):
  code_search calls-to -m MyApp.Accounts -f get_user # Shows file/line/column for each call")]
pub struct CallsToCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Function name (optional, if not specified shows all calls to module)
    #[arg(short = 'f', long)]
    pub function: Option<String>,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for CallsToCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
