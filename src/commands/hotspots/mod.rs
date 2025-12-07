mod execute;
mod output;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_hotspots_no_args() {
        let args = Args::try_parse_from(["code_search", "hotspots"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Incoming));
                assert!(cmd.module.is_none());
                assert_eq!(cmd.project, "default");
                assert!(!cmd.regex);
                assert_eq!(cmd.limit, 20);
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_kind_outgoing() {
        let args =
            Args::try_parse_from(["code_search", "hotspots", "--kind", "outgoing"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Outgoing));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_kind_total() {
        let args =
            Args::try_parse_from(["code_search", "hotspots", "--kind", "total"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Total));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_module_filter() {
        let args =
            Args::try_parse_from(["code_search", "hotspots", "--module", "MyApp"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert_eq!(cmd.module, Some("MyApp".to_string()));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_project() {
        let args = Args::try_parse_from([
            "code_search",
            "hotspots",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert_eq!(cmd.project, "my_app");
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "hotspots",
            "--module",
            "MyApp\\..*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_with_limit() {
        let args =
            Args::try_parse_from(["code_search", "hotspots", "--limit", "50"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_default_limit_is_20() {
        let args = Args::try_parse_from(["code_search", "hotspots"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert_eq!(cmd.limit, 20);
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_hotspots_limit_zero_rejected() {
        let result = Args::try_parse_from(["code_search", "hotspots", "--limit", "0"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_hotspots_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from(["code_search", "hotspots", "--limit", "1001"]);
        assert!(result.is_err());
    }
}
