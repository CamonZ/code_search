mod execute;
mod output;

use clap::Args;

/// Show struct fields, defaults, and types
///
/// Note: Named "struct_cmd" internally to avoid conflict with Rust's "struct" keyword
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search struct -m MyApp.User           # Show User struct definition
  code_search struct -m 'MyApp\\..*' -r      # All structs in MyApp namespace")]
pub struct StructCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module as a regular expression
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
    fn test_struct_requires_module() {
        let result = Args::try_parse_from(["code_search", "struct"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_struct_with_module() {
        let args = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Struct(cmd) => {
                assert_eq!(cmd.module, "MyApp.User");
            }
            _ => panic!("Expected Struct command"),
        }
    }

    #[rstest]
    fn test_struct_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp\\..*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Struct(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected Struct command"),
        }
    }

    #[rstest]
    fn test_struct_with_project() {
        let args = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Struct(cmd) => {
                assert_eq!(cmd.project, "my_app".to_string());
            }
            _ => panic!("Expected Struct command"),
        }
    }

    #[rstest]
    fn test_struct_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Struct(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Struct command"),
        }
    }

    #[rstest]
    fn test_struct_default_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Struct(cmd) => {
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Struct command"),
        }
    }

    #[rstest]
    fn test_struct_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_struct_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "struct",
            "--module",
            "MyApp.User",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
