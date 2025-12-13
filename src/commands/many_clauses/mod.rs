mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find functions with many pattern-matched heads
///
/// Functions with many clauses are those with multiple pattern-matched definitions,
/// indicating high branching complexity. These typically indicate functions that
/// should be broken down or simplified.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search many-clauses                   # Find functions with 5+ clauses
  code_search many-clauses --min-clauses 10 # Find functions with 10+ clauses
  code_search many-clauses -m MyApp.Web      # Filter to MyApp.Web namespace
  code_search many-clauses --include-generated # Include macro-generated functions
  code_search many-clauses -l 20             # Show top 20 functions with most clauses
")]
pub struct ManyClausesCmd {
    /// Minimum clauses to be considered
    #[arg(long, default_value = "5")]
    pub min_clauses: i64,

    /// Include macro-generated functions (excluded by default)
    #[arg(long)]
    pub include_generated: bool,

    /// Module filter pattern (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ManyClausesCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
