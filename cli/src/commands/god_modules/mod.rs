mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find god modules - modules with high function count and high connectivity
///
/// God modules are those with many functions and high incoming/outgoing call counts,
/// indicating they have too many responsibilities.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search god-modules                         # Find all god modules
  code_search god-modules MyApp.Core              # Filter to MyApp.Core namespace
  code_search god-modules --min-functions 30      # With minimum 30 functions
  code_search god-modules --min-loc 500           # With minimum 500 lines of code
  code_search god-modules --min-total 15          # With minimum 15 total connectivity
  code_search god-modules -l 20                   # Show top 20 god modules
")]
pub struct GodModulesCmd {
    /// Module filter pattern (substring match by default, regex with --regex)
    pub module: Option<String>,

    /// Minimum function count to be considered a god module
    #[arg(long, default_value = "20")]
    pub min_functions: i64,

    /// Minimum lines of code to be considered a god module
    #[arg(long, default_value = "0")]
    pub min_loc: i64,

    /// Minimum total connectivity (incoming + outgoing) to be considered a god module
    #[arg(long, default_value = "10")]
    pub min_total: i64,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for GodModulesCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
