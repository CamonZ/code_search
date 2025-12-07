mod execute;
mod output;

use clap::Args;

/// Show what modules depend on a given module (incoming module dependencies)
#[derive(Args, Debug)]
pub struct DependedByCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in (default: all projects)
    #[arg(long)]
    pub project: Option<String>,

    /// Treat module as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of dependents to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_depended_by_requires_module() {
        let result = Args::try_parse_from(["code_search", "depended-by"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depended_by_with_module() {
        let args = Args::try_parse_from([
            "code_search",
            "depended-by",
            "--module",
            "MyApp.Repo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependedBy(cmd) => {
                assert_eq!(cmd.module, "MyApp.Repo");
                assert!(!cmd.regex);
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected DependedBy command"),
        }
    }

    #[rstest]
    fn test_depended_by_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "depended-by",
            "--module",
            "MyApp\\..*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependedBy(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected DependedBy command"),
        }
    }

    #[rstest]
    fn test_depended_by_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "depended-by",
            "--module",
            "MyApp.Repo",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependedBy(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected DependedBy command"),
        }
    }

    #[rstest]
    fn test_depended_by_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "depended-by",
            "--module",
            "MyApp.Repo",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depended_by_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "depended-by",
            "--module",
            "MyApp.Repo",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
