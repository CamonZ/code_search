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

/// Show what modules a given module depends on (outgoing module dependencies)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search depends-on MyApp.Accounts       # What does Accounts depend on?
  code_search depends-on 'MyApp\\.Web.*' -r   # Dependencies of Web modules")]
pub struct DependsOnCmd {
    /// Module name (exact match or pattern with --regex)
    pub module: String,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for DependsOnCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
