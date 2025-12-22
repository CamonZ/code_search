mod execute;
mod output;

use std::error::Error;

use clap::Args;
use db::DbInstance;

use crate::commands::{CommandRunner, CommonArgs, Execute};
use crate::output::{OutputFormat, Outputable};

/// Analyze module connectivity using namespace-based clustering
///
/// Groups modules by namespace hierarchy and measures internal vs external connectivity.
/// Shows cohesion metrics (internal / (internal + external)) for each cluster.
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search clusters                      # Show all namespace clusters
  code_search clusters MyApp.Core           # Filter to MyApp.Core namespace
  code_search clusters --depth 2            # Cluster at depth 2 (e.g., MyApp.Accounts)
  code_search clusters --depth 3            # Cluster at depth 3 (e.g., MyApp.Accounts.Auth)
  code_search clusters --show-dependencies  # Include cross-namespace call counts
")]
pub struct ClustersCmd {
    /// Module filter pattern (substring match by default, regex with --regex)
    pub module: Option<String>,

    /// Namespace depth for clustering (default: 2)
    #[arg(long, default_value = "2")]
    pub depth: usize,

    /// Show cross-namespace dependencies
    #[arg(long)]
    pub show_dependencies: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

impl CommandRunner for ClustersCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
