//! CLI parsing tests for trace command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "trace",
        test_name: test_requires_module,
        required_arg: "--module",
    }

    crate::cli_required_arg_test! {
        command: "trace",
        test_name: test_requires_function,
        required_arg: "--function",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_module_and_function,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: module,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_function_name,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_project,
        args: ["--module", "MyApp", "--function", "foo", "--project", "my_custom_project"],
        field: project,
        expected: "my_custom_project",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_depth,
        args: ["--module", "MyApp", "--function", "foo", "--depth", "10"],
        field: depth,
        expected: 10,
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_limit,
        args: ["--module", "MyApp", "--function", "foo", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "trace",
        variant: Trace,
        required_args: ["--module", "MyApp", "--function", "foo"],
        limit: {
            field: limit,
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
            "trace",
            "--module",
            "MyApp",
            "--function",
            "foo",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.depth, 5);
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_depth_zero_rejected() {
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
    fn test_depth_exceeds_max_rejected() {
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
}
