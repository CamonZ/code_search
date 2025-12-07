mod execute;
mod output;

use clap::Args;

#[derive(Args, Debug)]
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

    /// Project to search in (default: all projects)
    #[arg(long)]
    pub project: Option<String>,

    /// Treat module and function as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of results to return
    #[arg(short, long, default_value_t = 100)]
    pub limit: usize,
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
                assert_eq!(cmd.project, Some("my_app".to_string()));
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
}
