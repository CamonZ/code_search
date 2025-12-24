mod execute;
mod output;

use std::error::Error;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find large functions that may need refactoring
///
/// Large functions are those with many lines of code (large `end_line - start_line`).
/// These typically indicate functions that should be broken down into smaller pieces.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search large-functions                     # Find functions with 50+ lines
  code_search large-functions MyApp.Web           # Filter to MyApp.Web namespace
  code_search large-functions --min-lines 100     # Find functions with 100+ lines
  code_search large-functions --include-generated # Include macro-generated functions
  code_search large-functions -l 20               # Show top 20 largest functions
")]
pub struct LargeFunctionsCmd {
    /// Module filter pattern (substring match by default, regex with --regex)
    pub module: Option<String>,

    /// Minimum lines to be considered large
    #[arg(long, default_value = "50")]
    pub min_lines: i64,

    /// Include macro-generated functions (excluded by default)
    #[arg(long)]
    pub include_generated: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for LargeFunctionsCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
