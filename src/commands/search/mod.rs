mod execute;
mod output;

use clap::{Args, ValueEnum};

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
  code_search search -p User                 # Find modules containing 'User'
  code_search search -p get_ -k functions    # Find functions starting with 'get_'
  code_search search -p '^MyApp\\.API' -r    # Regex match for module prefix")]
pub struct SearchCmd {
    /// Pattern to search for (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub pattern: String,

    /// What to search for
    #[arg(short, long, value_enum, default_value_t = SearchKind::Modules)]
    pub kind: SearchKind,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,

    /// Treat pattern as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_search_requires_pattern() {
        let result = Args::try_parse_from(["code_search", "search"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--pattern"));
    }

    #[rstest]
    fn test_search_with_pattern() {
        let args = Args::try_parse_from(["code_search", "search", "--pattern", "User"]).unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert_eq!(cmd.pattern, "User");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_kind_default_is_modules() {
        let args = Args::try_parse_from(["code_search", "search", "--pattern", "test"]).unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert!(matches!(cmd.kind, SearchKind::Modules));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_kind_functions() {
        let args = Args::try_parse_from([
            "code_search",
            "search",
            "--pattern",
            "get_",
            "--kind",
            "functions",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert!(matches!(cmd.kind, SearchKind::Functions));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_with_project_filter() {
        let args = Args::try_parse_from([
            "code_search",
            "search",
            "--pattern",
            "User",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert_eq!(cmd.project, "my_app".to_string());
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "search",
            "--pattern",
            "User",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_default_limit() {
        let args = Args::try_parse_from(["code_search", "search", "--pattern", "User"]).unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "search",
            "--pattern",
            "User",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_search_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "search",
            "--pattern",
            "User",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
