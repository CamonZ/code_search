mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find functions accepting a specific type pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search accepts \"User.t\"              # Find functions accepting User.t
  code_search accepts \"map()\"               # Find functions accepting maps
  code_search accepts \"User.t\" MyApp        # Filter to module MyApp
  code_search accepts -r \"list\\(.*\\)\"     # Regex pattern matching
")]
pub struct AcceptsCmd {
    /// Type pattern to search for in input types
    pub pattern: String,

    /// Module filter pattern
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for AcceptsCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
