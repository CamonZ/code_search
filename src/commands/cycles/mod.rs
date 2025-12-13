mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Detect circular dependencies between modules
///
/// Analyzes the call graph to find cycles where modules directly or indirectly
/// depend on each other, creating circular imports or call loops.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search cycles                           # Find all cycles
  code_search cycles -m MyApp.Core             # Filter to MyApp.Core namespace
  code_search cycles --max-length 3            # Only show cycles of length <= 3
  code_search cycles --involving MyApp.Accounts # Only cycles involving Accounts")]
pub struct CyclesCmd {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Maximum cycle length to find
    #[arg(long)]
    pub max_length: Option<usize>,

    /// Only show cycles involving this module (substring match)
    #[arg(long)]
    pub involving: Option<String>,

    /// Module filter pattern (substring or regex with -r)
    #[arg(short, long)]
    pub module: Option<String>,
}

impl CommandRunner for CyclesCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
