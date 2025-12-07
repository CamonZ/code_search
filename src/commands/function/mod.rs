mod execute;
mod output;

use clap::Args;

#[derive(Args, Debug)]
pub struct FunctionCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

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
    fn test_function_requires_module_and_function() {
        let result = Args::try_parse_from(["code_search", "function"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_function_requires_function_arg() {
        let result = Args::try_parse_from(["code_search", "function", "--module", "MyApp"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_function_with_module_and_function() {
        let args = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Function(cmd) => {
                assert_eq!(cmd.module, "MyApp.Accounts");
                assert_eq!(cmd.function, "get_user");
                assert_eq!(cmd.arity, None);
            }
            _ => panic!("Expected Function command"),
        }
    }

    #[rstest]
    fn test_function_with_arity() {
        let args = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
            "--arity",
            "1",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Function(cmd) => {
                assert_eq!(cmd.arity, Some(1));
            }
            _ => panic!("Expected Function command"),
        }
    }

    #[rstest]
    fn test_function_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp.*",
            "--function",
            "get_.*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Function(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected Function command"),
        }
    }

    #[rstest]
    fn test_function_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Function(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Function command"),
        }
    }

    #[rstest]
    fn test_function_default_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp",
            "--function",
            "foo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Function(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Function command"),
        }
    }

    #[rstest]
    fn test_function_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_function_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "function",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
