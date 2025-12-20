//! CLI parsing tests for location command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Required argument tests
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "location",
        test_name: test_requires_function,
        required_arg: "<FUNCTION>",
    }

    // =========================================================================
    // Option tests
    // =========================================================================

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_function_only,
        args: ["get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_module,
        args: ["get_user", "MyApp.Accounts"],
        field: module,
        expected: Some("MyApp.Accounts".to_string()),
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_arity,
        args: ["get_user", "MyApp.Accounts", "--arity", "1"],
        field: arity,
        expected: Some(1),
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_regex,
        args: ["get_.*", "MyApp.*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_project,
        args: ["get_user", "MyApp.Accounts", "--project", "my_app"],
        field: common.project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_limit,
        args: ["get_user", "--limit", "10"],
        field: common.limit,
        expected: 10,
    }

    // =========================================================================
    // Limit validation tests
    // =========================================================================

    crate::cli_limit_tests! {
        command: "location",
        variant: Location,
        required_args: ["get_user"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
