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

/// Show what modules depend on a given module (incoming module dependencies)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search depended-by MyApp.Repo          # Who depends on Repo?
  code_search depended-by 'Ecto\\..*' -r      # Who depends on Ecto modules?")]
pub struct DependedByCmd {
    /// Module name (exact match or pattern with --regex)
    pub module: String,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for DependedByCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
