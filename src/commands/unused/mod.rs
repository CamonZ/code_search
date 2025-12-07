mod execute;
mod output;

use clap::Args;

/// Find functions that are never called
#[derive(Args, Debug)]
pub struct UnusedCmd {
    /// Module pattern to filter results (substring match by default, regex with --regex)
    #[arg(short, long)]
    pub module: Option<String>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module pattern as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Only show private functions (defp, defmacrop)
    #[arg(short, long, default_value_t = false, conflicts_with = "public_only")]
    pub private_only: bool,

    /// Only show public functions (def, defmacro)
    #[arg(short = 'P', long, default_value_t = false, conflicts_with = "private_only")]
    pub public_only: bool,

    /// Exclude compiler-generated functions (__struct__, __using__, __before_compile__, etc.)
    #[arg(short = 'x', long, default_value_t = false)]
    pub exclude_generated: bool,

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
    fn test_unused_no_args() {
        let args = Args::try_parse_from(["code_search", "unused"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.module.is_none());
                assert_eq!(cmd.project, "default");
                assert!(!cmd.regex);
                assert!(!cmd.private_only);
                assert!(!cmd.public_only);
                assert!(!cmd.exclude_generated);
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_module_filter() {
        let args =
            Args::try_parse_from(["code_search", "unused", "--module", "MyApp"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert_eq!(cmd.module, Some("MyApp".to_string()));
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_project() {
        let args = Args::try_parse_from([
            "code_search",
            "unused",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert_eq!(cmd.project, "my_app");
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "unused",
            "--module",
            "MyApp\\..*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_limit() {
        let args =
            Args::try_parse_from(["code_search", "unused", "--limit", "50"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_limit_zero_rejected() {
        let result = Args::try_parse_from(["code_search", "unused", "--limit", "0"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_unused_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from(["code_search", "unused", "--limit", "1001"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_unused_with_private_only() {
        let args = Args::try_parse_from(["code_search", "unused", "--private-only"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.private_only);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_exclude_generated() {
        let args = Args::try_parse_from(["code_search", "unused", "--exclude-generated"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.exclude_generated);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_short_flags() {
        let args = Args::try_parse_from(["code_search", "unused", "-p", "-x"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.private_only);
                assert!(cmd.exclude_generated);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_public_only() {
        let args = Args::try_parse_from(["code_search", "unused", "--public-only"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.public_only);
                assert!(!cmd.private_only);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_with_public_only_short() {
        let args = Args::try_parse_from(["code_search", "unused", "-P"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.public_only);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_unused_private_and_public_conflict() {
        let result = Args::try_parse_from(["code_search", "unused", "--private-only", "--public-only"]);
        assert!(result.is_err());
    }
}
