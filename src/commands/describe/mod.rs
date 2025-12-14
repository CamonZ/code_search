mod descriptions;
mod execute;
mod output;

use std::error::Error;

use clap::Args;

use crate::commands::{CommandRunner, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

/// Display detailed documentation about available commands
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search describe                  # List all available commands
  code_search describe calls-to         # Detailed info about calls-to command
  code_search describe calls-to calls-from trace  # Describe multiple commands")]
pub struct DescribeCmd {
    /// Command(s) to describe (if empty, lists all)
    pub commands: Vec<String>,
}

impl CommandRunner for DescribeCmd {
    fn run(self, _db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(_db)?;
        Ok(result.format(format))
    }
}
