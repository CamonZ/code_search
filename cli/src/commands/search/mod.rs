mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::{Args, ValueEnum};
use db::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// What to search for
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum SearchKind {
    /// Search for modules
    #[default]
    Modules,
    /// Search for functions
    Functions,
}

/// Search for modules or functions by name pattern
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search search User                    # Find modules containing 'User'
  code_search search get_ -k functions       # Find functions starting with 'get_'
  code_search search -r '^MyApp\\.API'       # Regex match for module prefix
")]
pub struct SearchCmd {
    /// Pattern to search for (substring match by default, regex with --regex)
    pub pattern: String,

    /// What to search for
    #[arg(short, long, value_enum, default_value_t = SearchKind::Modules)]
    pub kind: SearchKind,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for SearchCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
