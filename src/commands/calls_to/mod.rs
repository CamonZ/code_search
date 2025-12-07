mod execute;
mod output;

use clap::Args;

#[derive(Args, Debug)]
pub struct CallsToCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Function name (optional, if not specified shows all calls to module)
    #[arg(short = 'f', long)]
    pub function: Option<String>,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

    /// Project to search in (default: all projects)
    #[arg(long)]
    pub project: Option<String>,

    /// Treat module and function as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_calls_to_requires_module() {
        let result = Args::try_parse_from(["code_search", "calls-to"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_calls_to_with_module_only() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsTo(cmd) => {
                assert_eq!(cmd.module, "MyApp.Repo");
                assert_eq!(cmd.function, None);
            }
            _ => panic!("Expected CallsTo command"),
        }
    }

    #[rstest]
    fn test_calls_to_with_function() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
            "--function",
            "get",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsTo(cmd) => {
                assert_eq!(cmd.module, "MyApp.Repo");
                assert_eq!(cmd.function, Some("get".to_string()));
            }
            _ => panic!("Expected CallsTo command"),
        }
    }

    #[rstest]
    fn test_calls_to_with_arity() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
            "--function",
            "get",
            "--arity",
            "2",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsTo(cmd) => {
                assert_eq!(cmd.arity, Some(2));
            }
            _ => panic!("Expected CallsTo command"),
        }
    }

    #[rstest]
    fn test_calls_to_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp\\.Repo",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsTo(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected CallsTo command"),
        }
    }

    #[rstest]
    fn test_calls_to_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
            "--limit",
            "25",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsTo(cmd) => {
                assert_eq!(cmd.limit, 25);
            }
            _ => panic!("Expected CallsTo command"),
        }
    }

    #[rstest]
    fn test_calls_to_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_calls_to_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "calls-to",
            "--module",
            "MyApp.Repo",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
