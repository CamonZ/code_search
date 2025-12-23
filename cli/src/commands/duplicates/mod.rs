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

/// Find functions with identical or near-identical implementations
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search duplicates                  # Find all duplicate functions
  code_search duplicates MyApp            # Filter to specific module
  code_search duplicates --by-module      # Rank modules by duplication
  code_search duplicates --exact          # Use exact source matching
  code_search duplicates --exclude-generated  # Exclude macro-generated functions")]
pub struct DuplicatesCmd {
    /// Module filter pattern (substring match by default, regex with -r)
    pub module: Option<String>,

    /// Aggregate results by module (show which modules have most duplicates)
    #[arg(long)]
    pub by_module: bool,

    /// Use exact source matching instead of AST matching
    #[arg(long)]
    pub exact: bool,

    /// Exclude macro-generated functions
    #[arg(long)]
    pub exclude_generated: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for DuplicatesCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
