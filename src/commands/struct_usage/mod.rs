mod execute;
mod output;

use std::error::Error;

use clap::Args;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

/// Find functions that accept or return a specific type pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search struct-usage \"User.t\"         # Find functions using User.t
  code_search struct-usage \"Changeset.t\"    # Find functions using Changeset.t
  code_search struct-usage \"User.t\" -m MyApp # Filter to module MyApp
  code_search struct-usage -r \".*\\.t\"      # Regex pattern matching")]
pub struct StructUsageCmd {
    /// Type pattern to search for in both inputs and returns
    pub pattern: String,

    /// Module filter pattern
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for StructUsageCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
