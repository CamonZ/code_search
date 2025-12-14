mod execute;
mod output;

use std::error::Error;

use clap::Args;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

/// Show which modules work with a given struct type
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search struct-modules \"User.t\"         # Show modules using User.t
  code_search struct-modules \"Changeset.t\"    # Show modules using Changeset.t
  code_search struct-modules \"User.t\" -m MyApp # Filter to module MyApp
  code_search struct-modules -r \".*\\.t\"      # Regex pattern matching")]
pub struct StructModulesCmd {
    /// Struct type pattern to search for
    pub pattern: String,

    /// Module filter pattern
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for StructModulesCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
