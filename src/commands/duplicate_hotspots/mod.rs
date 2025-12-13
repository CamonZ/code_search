mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find modules with the most duplicated functions
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search duplicate-hotspots             # Find modules with most duplicates
  code_search duplicate-hotspots -m MyApp    # Filter to specific module
  code_search duplicate-hotspots --exact      # Use exact source matching
  code_search duplicate-hotspots -m 'Web' --regex # Match module with regex")]
pub struct DuplicateHotspotsCmd {
    /// Module filter pattern (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    /// Use exact source matching instead of AST matching
    #[arg(long)]
    pub exact: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for DuplicateHotspotsCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
