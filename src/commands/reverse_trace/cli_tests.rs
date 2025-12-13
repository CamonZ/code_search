//! CLI parsing tests for reverse-trace command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "reverse-trace",
        test_name: test_requires_module,
        required_arg: "--module",
    }

    crate::cli_required_arg_test! {
        command: "reverse-trace",
        test_name: test_requires_function,
        required_arg: "--function",
    }

    crate::cli_option_test! {
        command: "reverse-trace",
        variant: ReverseTrace,
        test_name: test_with_module_and_function,
        args: ["--module", "MyApp.Repo", "--function", "get"],
        field: module,
        expected: "MyApp.Repo",
    }

    crate::cli_option_test! {
        command: "reverse-trace",
        variant: ReverseTrace,
        test_name: test_function_name,
        args: ["--module", "MyApp.Repo", "--function", "get"],
        field: function,
        expected: "get",
    }

    crate::cli_option_test! {
        command: "reverse-trace",
        variant: ReverseTrace,
        test_name: test_with_depth,
        args: ["--module", "MyApp", "--function", "foo", "--depth", "10"],
        field: depth,
        expected: 10,
    }

    crate::cli_option_test! {
        command: "reverse-trace",
        variant: ReverseTrace,
        test_name: test_with_limit,
        args: ["--module", "MyApp", "--function", "foo", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "reverse-trace",
        variant: ReverseTrace,
        required_args: ["--module", "MyApp", "--function", "foo"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Edge case tests (depth validation - different from standard limit)
    // =========================================================================

    #[rstest]
    fn test_depth_default() {
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
                assert_eq!(cmd.depth, 5);
            }
            _ => panic!("Expected ReverseTrace command"),
        }
    }

    #[rstest]
    fn test_depth_zero_rejected() {
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
    fn test_depth_exceeds_max_rejected() {
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
}
