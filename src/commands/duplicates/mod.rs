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

/// Find functions with identical or near-identical implementations
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search duplicates                  # Find all duplicate functions
  code_search duplicates -m MyApp         # Filter to specific module
  code_search duplicates --exact          # Use exact source matching
  code_search duplicates -m 'App' --regex # Match module with regex")]
pub struct DuplicatesCmd {
    /// Module filter pattern (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    /// Use exact source matching instead of AST matching
    #[arg(long)]
    pub exact: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for DuplicatesCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
