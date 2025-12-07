mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::{Args, ValueEnum};

/// What type of hotspots to find
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum HotspotKind {
    /// Functions with most incoming calls (most called)
    #[default]
    Incoming,
    /// Functions with most outgoing calls (calls many things)
    Outgoing,
    /// Functions with highest total (incoming + outgoing)
    Total,
}

/// Find functions with the most incoming/outgoing calls
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search hotspots                       # Most called functions (incoming)
  code_search hotspots -k outgoing           # Functions that call many others
  code_search hotspots -k total              # Highest total connections
  code_search hotspots -m MyApp -l 10        # Top 10 in MyApp namespace")]
pub struct HotspotsCmd {
    /// Type of hotspots to find
    #[arg(short, long, value_enum, default_value_t = HotspotKind::Incoming)]
    pub kind: HotspotKind,

    /// Module pattern to filter results (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module pattern as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
