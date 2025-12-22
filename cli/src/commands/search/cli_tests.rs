//! CLI parsing tests for search command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use crate::commands::search::SearchKind;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Required argument tests
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "search",
        test_name: test_search_requires_pattern,
        required_arg: "<PATTERN>",
    }

    // =========================================================================
    // Option tests
    // =========================================================================

    crate::cli_option_test! {
        command: "search",
        variant: Search,
        test_name: test_search_with_pattern,
        args: ["User"],
        field: pattern,
        expected: "User",
    }

    crate::cli_option_test! {
        command: "search",
        variant: Search,
        test_name: test_search_with_project_filter,
        args: ["User", "--project", "my_app"],
        field: common.project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "search",
        variant: Search,
        test_name: test_search_with_limit,
        args: ["User", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    // =========================================================================
    // Limit validation tests
    // =========================================================================

    crate::cli_limit_tests! {
        command: "search",
        variant: Search,
        required_args: ["User"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Edge case tests (kept as regular tests due to matches! macro usage)
    // =========================================================================

    #[rstest]
    fn test_search_kind_default_is_modules() {
        let args = Args::try_parse_from(["code_search", "search", "test"]).unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert!(matches!(cmd.kind, SearchKind::Modules));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[rstest]
    fn test_search_kind_functions() {
        let args =
            Args::try_parse_from(["code_search", "search", "get_", "--kind", "functions"]).unwrap();
        match args.command {
            crate::commands::Command::Search(cmd) => {
                assert!(matches!(cmd.kind, SearchKind::Functions));
            }
            _ => panic!("Expected Search command"),
        }
    }
}
