mod execute;
mod output;

use clap::Args;

/// Find where a function is defined (file:line_start:line_end)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search location -f get_user           # Find all get_user functions
  code_search location -m MyApp -f get_user  # In specific module
  code_search location -f get_user -a 1      # With specific arity
  code_search location -f 'get_.*' -r        # Regex pattern matching")]
pub struct LocationCmd {
    /// Module name (exact match or pattern with --regex). If not specified, searches all modules.
    #[arg(short, long)]
    pub module: Option<String>,

    /// Function name (exact match or pattern with --regex)
    #[arg(short = 'f', long)]
    pub function: String,

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
    fn test_location_requires_function() {
        let result = Args::try_parse_from(["code_search", "location"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_location_with_function_only() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.module, None);
                assert_eq!(cmd.function, "get_user");
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_with_module_and_function() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.module, Some("MyApp.Accounts".to_string()));
                assert_eq!(cmd.function, "get_user");
                assert_eq!(cmd.arity, None);
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_with_arity() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
            "--arity",
            "1",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.arity, Some(1));
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_with_regex_flag() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--module",
            "MyApp.*",
            "--function",
            "get_.*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_with_project() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.project, "my_app".to_string());
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--function",
            "get_user",
            "--limit",
            "10",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.limit, 10);
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_default_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "location",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Location(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Location command"),
        }
    }

    #[rstest]
    fn test_location_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "location",
            "--function",
            "get_user",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_location_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "location",
            "--function",
            "get_user",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
