//! CLI parsing tests for trace command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Required argument tests
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "trace",
        test_name: test_requires_module,
        required_arg: "<MODULE>",
    }

    crate::cli_required_arg_test! {
        command: "trace",
        test_name: test_requires_function,
        required_arg: "<FUNCTION>",
    }

    // =========================================================================
    // Option tests
    // =========================================================================

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_module_and_function,
        args: ["MyApp.Accounts", "get_user"],
        field: module,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_function_name,
        args: ["MyApp.Accounts", "get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_project,
        args: ["MyApp", "foo", "--project", "my_custom_project"],
        field: common.project,
        expected: "my_custom_project",
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_depth,
        args: ["MyApp", "foo", "--depth", "10"],
        field: depth,
        expected: 10,
    }

    crate::cli_option_test! {
        command: "trace",
        variant: Trace,
        test_name: test_with_limit,
        args: ["MyApp", "foo", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    // =========================================================================
    // Limit validation tests
    // =========================================================================

    crate::cli_limit_tests! {
        command: "trace",
        variant: Trace,
        required_args: ["MyApp", "foo"],
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
        let args = Args::try_parse_from(["code_search", "trace", "MyApp", "foo"]).unwrap();
        match args.command {
            crate::commands::Command::Trace(cmd) => {
                assert_eq!(cmd.depth, 5);
            }
            _ => panic!("Expected Trace command"),
        }
    }

    #[rstest]
    fn test_depth_zero_rejected() {
        let result =
            Args::try_parse_from(["code_search", "trace", "MyApp", "foo", "--depth", "0"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depth_exceeds_max_rejected() {
        let result =
            Args::try_parse_from(["code_search", "trace", "MyApp", "foo", "--depth", "21"]);
        assert!(result.is_err());
    }
}
