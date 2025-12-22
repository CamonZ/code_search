mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};
use crate::queries::hotspots::HotspotKind;

/// Find functions with the most incoming/outgoing calls
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search hotspots                       # Most called functions (incoming)
  code_search hotspots -k outgoing           # Functions that call many others
  code_search hotspots -k total              # Highest total connections
  code_search hotspots -k ratio              # Boundary functions (high incoming/outgoing ratio)
  code_search hotspots MyApp -l 10           # Top 10 in MyApp namespace
  code_search hotspots --exclude-generated   # Exclude macro-generated functions

  # Find wide functions (high fan-out):
  code_search hotspots -k outgoing -l 20     # Top 20 functions calling many others

  # Find deep functions (high fan-in):
  code_search hotspots -k incoming -l 20     # Top 20 most-called functions

  # Find boundary functions (many callers, few dependencies):
  code_search hotspots -k ratio -l 20        # Top 20 boundary functions")]
pub struct HotspotsCmd {
    /// Module pattern to filter results (substring match by default, regex with --regex)
    pub module: Option<String>,

    /// Type of hotspots to find
    #[arg(short, long, value_enum, default_value_t = HotspotKind::Incoming)]
    pub kind: HotspotKind,

    /// Exclude macro-generated functions
    #[arg(long)]
    pub exclude_generated: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for HotspotsCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
