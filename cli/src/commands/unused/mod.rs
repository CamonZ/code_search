mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find functions that are never called
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search unused                       # Find all unused functions
  code_search unused MyApp.Accounts        # Filter to specific module
  code_search unused -P                    # Unused public functions (entry points)
  code_search unused -p                    # Unused private functions (dead code)
  code_search unused -Px                   # Public only, exclude generated
  code_search unused 'Accounts.*' -r       # Match module with regex")]
pub struct UnusedCmd {
    /// Module pattern to filter results (substring match by default, regex with -r)
    pub module: Option<String>,

    /// Only show private functions (defp, defmacrop) - likely dead code
    #[arg(short, long, default_value_t = false, conflicts_with = "public_only")]
    pub private_only: bool,

    /// Only show public functions (def, defmacro) - potential entry points
    #[arg(
        short = 'P',
        long,
        default_value_t = false,
        conflicts_with = "private_only"
    )]
    pub public_only: bool,

    /// Exclude compiler-generated functions (__struct__, __info__, etc.)
    #[arg(short = 'x', long, default_value_t = false)]
    pub exclude_generated: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for UnusedCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
