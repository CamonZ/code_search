mod execute;
mod output;

use clap::Args;

/// Trace call chains from a starting function (forward traversal)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search trace -m MyApp.Web -f index            # Trace from controller action
  code_search trace -m MyApp -f handle_call --depth 10   # Deeper traversal
  code_search trace -m 'MyApp\\..*' -f 'handle_.*' -r    # Regex pattern")]
pub struct TraceCmd {
    /// Starting module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Starting function name (exact match or pattern with --regex)
    #[arg(short = 'f', long)]
    pub function: String,

    /// Function arity (optional)
    #[arg(short, long)]
    pub arity: Option<i64>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module and function as regular expressions
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum depth to traverse (1-20)
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub depth: u32,

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
    fn test_trace_requires_module_and_function() {
        let result = Args::try_parse_from(["code_search", "trace"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_trace_requires_function() {
        let result = Args::try_parse_from(["code_search", "trace", "--module", "MyApp"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_trace_with_module_and_function() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp.Accounts",
            "--function",
            "get_user",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.module, "MyApp.Accounts");
                assert_eq!(cmd.function, "get_user");
                assert_eq!(cmd.depth, 5); // default
                assert_eq!(cmd.project, "default"); // default project
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_project_defaults_to_default() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.project, "default");
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_project_can_be_overridden() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--project",
            "my_custom_project",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.project, "my_custom_project");
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_with_depth() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--depth",
            "10",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.depth, 10);
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_depth_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--depth",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_trace_depth_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--depth",
            "21",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_trace_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_default_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_trace_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "trace",
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
    fn test_trace_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "trace",
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
