mod execute;
mod output;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find boundary modules - modules with high fan-in but low fan-out
///
/// Boundary modules are those that many other modules depend on but have few
/// dependencies themselves. They are identified by high ratio of incoming to
/// outgoing calls, indicating they are central points in the architecture.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search boundaries                          # Find all boundary modules
  code_search boundaries --min-incoming 5         # With minimum 5 incoming calls
  code_search boundaries --min-ratio 2.0          # With minimum 2.0 ratio
  code_search boundaries -m MyApp.Web             # Filter to MyApp.Web namespace
  code_search boundaries -l 20                    # Show top 20 boundary modules
")]
pub struct BoundariesCmd {
    /// Minimum incoming calls to be considered a boundary module
    #[arg(long, default_value = "1")]
    pub min_incoming: i64,

    /// Minimum ratio (incoming/outgoing) to be considered a boundary module
    #[arg(long, default_value = "2.0")]
    pub min_ratio: f64,

    /// Module filter pattern (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for BoundariesCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
