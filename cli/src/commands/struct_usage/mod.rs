mod execute;
mod output;

#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod execute_tests;
#[cfg(test)]
mod output_tests;

use std::error::Error;

use clap::Args;
use db::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find functions that accept or return a specific type pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search struct-usage \"User.t\"             # Find functions using User.t
  code_search struct-usage \"Changeset.t\"        # Find functions using Changeset.t
  code_search struct-usage \"User.t\" MyApp       # Filter to module MyApp
  code_search struct-usage \"User.t\" --by-module # Summarize by module
  code_search struct-usage -r \".*\\.t\"          # Regex pattern matching
")]
pub struct StructUsageCmd {
    /// Type pattern to search for in both inputs and returns
    pub pattern: String,

    /// Module filter pattern
    pub module: Option<String>,

    /// Aggregate results by module (show counts instead of function details)
    #[arg(long)]
    pub by_module: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for StructUsageCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
