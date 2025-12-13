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
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Display complexity metrics for functions
///
/// Shows functions with complexity scores and nesting depths.
/// Complexity is a measure of the cyclomatic complexity of a function,
/// and nesting depth is the maximum depth of nested control structures.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search complexity                    # Show all functions with complexity >= 1
  code_search complexity --min 10           # Show functions with complexity >= 10
  code_search complexity --min-depth 3      # Show functions with nesting depth >= 3
  code_search complexity -m MyApp.Accounts  # Filter to MyApp.Accounts module
  code_search complexity --exclude-generated # Exclude macro-generated functions
  code_search complexity -l 20              # Show top 20 most complex functions
")]
pub struct ComplexityCmd {
    /// Minimum complexity threshold
    #[arg(long, default_value = "1")]
    pub min: i64,

    /// Minimum nesting depth threshold
    #[arg(long, default_value = "0")]
    pub min_depth: i64,

    /// Exclude macro-generated functions
    #[arg(long)]
    pub exclude_generated: bool,

    /// Module filter pattern (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ComplexityCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
