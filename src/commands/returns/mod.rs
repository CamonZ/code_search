mod execute;
mod output;

use std::error::Error;

use clap::Args;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

/// Find functions returning a specific type pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search returns \"User.t\"              # Find functions returning User.t
  code_search returns \"nil\"                 # Find functions returning nil
  code_search returns \"{:error\" -m MyApp    # Filter to module MyApp
  code_search returns -r \"list\\(.*\\)\"     # Regex pattern matching")]
pub struct ReturnsCmd {
    /// Type pattern to search for in return types
    pub pattern: String,

    /// Module filter pattern
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ReturnsCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
