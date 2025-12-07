mod cli_tests;
mod execute;
mod models;
mod output;

use std::path::PathBuf;

use clap::Args;

const DEFAULT_PROJECT: &str = "default";

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
  code_search import -f call_graph.json      # Import with default project name
  code_search import -f cg.json -p my_app    # Import into 'my_app' project
  code_search import -f cg.json --clear      # Clear DB before importing")]
pub struct ImportCmd {
    /// Path to the call graph JSON file
    #[arg(short, long, value_parser = validate_file_exists)]
    pub file: PathBuf,
    /// Project name for namespacing (allows multiple projects in same DB)
    #[arg(short, long, default_value = DEFAULT_PROJECT)]
    pub project: String,
    /// Clear all existing data before import (or just project data if --project is set)
    #[arg(long, default_value_t = false)]
    pub clear: bool,
}
