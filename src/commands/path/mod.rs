mod execute;
mod output;

use clap::Args;

/// Find a call path between two functions
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search path --from-module MyApp.Web --from-function index \\
                   --to-module MyApp.Repo --to-function get
  code_search path --from-module MyApp.API --from-function create \\
                   --to-module Ecto.Repo --to-function insert --depth 15")]
pub struct PathCmd {
    /// Source module name
    #[arg(long)]
    pub from_module: String,

    /// Source function name
    #[arg(long)]
    pub from_function: String,

    /// Source function arity (optional)
    #[arg(long)]
    pub from_arity: Option<i64>,

    /// Target module name
    #[arg(long)]
    pub to_module: String,

    /// Target function name
    #[arg(long)]
    pub to_function: String,

    /// Target function arity (optional)
    #[arg(long)]
    pub to_arity: Option<i64>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Maximum depth to search (1-20)
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub depth: u32,

    /// Maximum number of paths to return (1-100)
    #[arg(short, long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_path_requires_all_args() {
        let result = Args::try_parse_from(["code_search", "path"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_path_requires_to_args() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp.Controller",
            "--from-function",
            "index",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_path_with_all_required_args() {
        let args = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp.Controller",
            "--from-function",
            "index",
            "--to-module",
            "MyApp.Repo",
            "--to-function",
            "get",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Path(cmd) => {
                assert_eq!(cmd.from_module, "MyApp.Controller");
                assert_eq!(cmd.from_function, "index");
                assert_eq!(cmd.to_module, "MyApp.Repo");
                assert_eq!(cmd.to_function, "get");
                assert_eq!(cmd.depth, 10); // default
                assert_eq!(cmd.limit, 10); // default
            }
            _ => panic!("Expected Path command"),
        }
    }

    #[rstest]
    fn test_path_with_arities() {
        let args = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp.Controller",
            "--from-function",
            "index",
            "--from-arity",
            "2",
            "--to-module",
            "MyApp.Repo",
            "--to-function",
            "get",
            "--to-arity",
            "2",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Path(cmd) => {
                assert_eq!(cmd.from_arity, Some(2));
                assert_eq!(cmd.to_arity, Some(2));
            }
            _ => panic!("Expected Path command"),
        }
    }

    #[rstest]
    fn test_path_with_depth() {
        let args = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--depth",
            "15",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Path(cmd) => {
                assert_eq!(cmd.depth, 15);
            }
            _ => panic!("Expected Path command"),
        }
    }

    #[rstest]
    fn test_path_depth_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--depth",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_path_depth_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--depth",
            "21",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_path_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--limit",
            "5",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Path(cmd) => {
                assert_eq!(cmd.limit, 5);
            }
            _ => panic!("Expected Path command"),
        }
    }

    #[rstest]
    fn test_path_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_path_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--limit",
            "101",
        ]);
        assert!(result.is_err());
    }
}
