mod execute;
mod output;

use clap::Args;

/// Show all functions defined in a file
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search file -f lib/accounts.ex        # Functions in specific file
  code_search file -f accounts               # Files containing 'accounts'
  code_search file -f 'lib/.*_test.ex' -r    # All test files with regex")]
pub struct FileCmd {
    /// File path pattern (substring match by default, regex with --regex)
    #[arg(short = 'f', long)]
    pub file: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat file path as a regular expression
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
    fn test_file_requires_file_arg() {
        let result = Args::try_parse_from(["code_search", "file"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_file_with_file_path() {
        let args = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/accounts.ex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::File(cmd) => {
                assert_eq!(cmd.file, "lib/accounts.ex");
                assert_eq!(cmd.project, "default");
                assert!(!cmd.regex);
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected File command"),
        }
    }

    #[rstest]
    fn test_file_with_project() {
        let args = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/accounts.ex",
            "--project",
            "my_app",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::File(cmd) => {
                assert_eq!(cmd.project, "my_app");
            }
            _ => panic!("Expected File command"),
        }
    }

    #[rstest]
    fn test_file_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/.*\\.ex$",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::File(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected File command"),
        }
    }

    #[rstest]
    fn test_file_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/accounts.ex",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::File(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected File command"),
        }
    }

    #[rstest]
    fn test_file_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/accounts.ex",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_file_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "file",
            "--file",
            "lib/accounts.ex",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
