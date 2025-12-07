mod execute;
mod output;

use clap::Args;

/// Trace call chains backwards - who calls the callers of a target
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search reverse-trace -m MyApp.Repo -f get     # Who ultimately calls Repo.get?
  code_search reverse-trace -m Ecto.Repo -f insert --depth 10  # Deeper traversal
  code_search reverse-trace -m MyApp -f 'handle_.*' -r   # Regex pattern")]
pub struct ReverseTraceCmd {
    /// Target module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Target function name (exact match or pattern with --regex)
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
    fn test_reverse_trace_requires_module_and_function() {
        let result = Args::try_parse_from(["code_search", "reverse-trace"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_reverse_trace_requires_function() {
        let result = Args::try_parse_from(["code_search", "reverse-trace", "--module", "MyApp"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_reverse_trace_with_module_and_function() {
        let args = Args::try_parse_from([
            "code_search",
            "reverse-trace",
            "--module",
            "MyApp.Repo",
            "--function",
            "get",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::ReverseTrace(cmd) => {
                assert_eq!(cmd.module, "MyApp.Repo");
                assert_eq!(cmd.function, "get");
                assert_eq!(cmd.depth, 5); // default
            }
            _ => panic!("Expected ReverseTrace command"),
        }
    }

    #[rstest]
    fn test_reverse_trace_with_depth() {
        let args = Args::try_parse_from([
            "code_search",
            "reverse-trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--depth",
            "10",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::ReverseTrace(cmd) => {
                assert_eq!(cmd.depth, 10);
            }
            _ => panic!("Expected ReverseTrace command"),
        }
    }

    #[rstest]
    fn test_reverse_trace_depth_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "reverse-trace",
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
    fn test_reverse_trace_depth_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "reverse-trace",
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
    fn test_reverse_trace_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "reverse-trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::ReverseTrace(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected ReverseTrace command"),
        }
    }

    #[rstest]
    fn test_reverse_trace_default_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "reverse-trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::ReverseTrace(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected ReverseTrace command"),
        }
    }

    #[rstest]
    fn test_reverse_trace_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "reverse-trace",
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
    fn test_reverse_trace_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "reverse-trace",
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
