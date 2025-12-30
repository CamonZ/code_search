mod execute;
mod output;

use std::error::Error;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find functions returning a specific type pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search returns \"User.t\"              # Find functions returning User.t
  code_search returns \"nil\"                 # Find functions returning nil
  code_search returns \"{:error\" MyApp       # Filter to module MyApp
  code_search returns -r \"list\\(.*\\)\"     # Regex pattern matching
")]
pub struct ReturnsCmd {
    /// Type pattern to search for in return types
    pub pattern: String,

    /// Module filter pattern
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ReturnsCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
