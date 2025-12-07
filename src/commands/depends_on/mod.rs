mod execute;
mod output;

use clap::Args;

/// Show what modules a given module depends on (outgoing module dependencies)
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search depends-on -m MyApp.Accounts       # What does Accounts depend on?
  code_search depends-on -m 'MyApp\\.Web.*' -r   # Dependencies of Web modules")]
pub struct DependsOnCmd {
    /// Module name (exact match or pattern with --regex)
    #[arg(short, long)]
    pub module: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Treat module as a regular expression
    #[arg(short, long, default_value_t = false)]
    pub regex: bool,

    /// Maximum number of dependencies to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    #[rstest]
    fn test_depends_on_requires_module() {
        let result = Args::try_parse_from(["code_search", "depends-on"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depends_on_with_module() {
        let args = Args::try_parse_from([
            "code_search",
            "depends-on",
            "--module",
            "MyApp.Accounts",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependsOn(cmd) => {
                assert_eq!(cmd.module, "MyApp.Accounts");
                assert!(!cmd.regex);
                assert_eq!(cmd.limit, 100);
            }
            _ => panic!("Expected DependsOn command"),
        }
    }

    #[rstest]
    fn test_depends_on_with_regex() {
        let args = Args::try_parse_from([
            "code_search",
            "depends-on",
            "--module",
            "MyApp\\..*",
            "--regex",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependsOn(cmd) => {
                assert!(cmd.regex);
            }
            _ => panic!("Expected DependsOn command"),
        }
    }

    #[rstest]
    fn test_depends_on_with_limit() {
        let args = Args::try_parse_from([
            "code_search",
            "depends-on",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "50",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::DependsOn(cmd) => {
                assert_eq!(cmd.limit, 50);
            }
            _ => panic!("Expected DependsOn command"),
        }
    }

    #[rstest]
    fn test_depends_on_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "depends-on",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depends_on_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "depends-on",
            "--module",
            "MyApp.Accounts",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
