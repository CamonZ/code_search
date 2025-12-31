mod cli_tests;
mod execute;
mod output;
mod output_tests;

use std::error::Error;
use std::path::PathBuf;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, Execute};
use crate::output::{OutputFormat, Outputable};

fn validate_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("File not found: {}", path.display()))
    }
}

/// Import a call graph JSON file into the database
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search code import -f call_graph.json      # Import call graph into database
  code_search code import -f cg.json --clear      # Clear DB before importing")]
pub struct ImportCmd {
    /// Path to the call graph JSON file
    #[arg(short, long, value_parser = validate_file_exists)]
    pub file: PathBuf,
    /// Clear all existing data before import
    #[arg(long, default_value_t = false)]
    pub clear: bool,
}

impl CommandRunner for ImportCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
