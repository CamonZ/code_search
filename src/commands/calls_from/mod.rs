mod execute;
mod output;

use clap::Args;

/// Show what a module/function calls (outgoing edges)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search calls-from -m MyApp.Accounts           # All calls from module
  code_search calls-from -m MyApp -f get_user        # Calls from specific function
  code_search calls-from -m MyApp -f get_user -a 1   # With specific arity")]
pub struct CallsFromCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Function name (optional, if not specified shows all calls from module)
    #[arg(short = 'f', long)]
    pub function: Option<String>,

    /// Function arity (optional, matches all arities if not specified)
    #[arg(short, long)]
    pub arity: Option<i64>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

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
    fn test_calls_from_requires_module() {
        let result = Args::try_parse_from(["code_search", "calls-from"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_calls_from_with_module_only() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsFrom(cmd) => {
                assert_eq!(cmd.module, "MyApp.Accounts");
                assert_eq!(cmd.function, None);
            }
            _ => panic!("Expected CallsFrom command"),
        }
    }

    #[rstest]
    fn test_calls_from_with_function() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsFrom(cmd) => {
                assert_eq!(cmd.module, "MyApp.Accounts");
                assert_eq!(cmd.function, Some("get_user".to_string()));
            }
            _ => panic!("Expected CallsFrom command"),
        }
    }

    #[rstest]
    fn test_calls_from_with_arity() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
            "--arity",
            "1",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsFrom(cmd) => {
                assert_eq!(cmd.arity, Some(1));
            }
            _ => panic!("Expected CallsFrom command"),
        }
    }

    #[rstest]
    fn test_calls_from_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsFrom(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected CallsFrom command"),
        }
    }

    #[rstest]
    fn test_calls_from_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::CallsFrom(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected CallsFrom command"),
        }
    }

    #[rstest]
    fn test_calls_from_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_calls_from_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "calls-from",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
