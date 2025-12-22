//! CLI parsing tests for function command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Required argument tests
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "function",
        test_name: test_requires_module,
        required_arg: "<MODULE>",
    }

    crate::cli_required_arg_test! {
        command: "function",
        test_name: test_requires_function,
        required_arg: "<FUNCTION>",
    }

    // =========================================================================
    // Option tests
    // =========================================================================

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_module_and_function,
        args: ["MyApp.Accounts", "get_user"],
        field: module,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_function_name,
        args: ["MyApp.Accounts", "get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_arity,
        args: ["MyApp.Accounts", "get_user", "--arity", "1"],
        field: arity,
        expected: Some(1),
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_regex,
        args: ["MyApp.*", "get_.*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_limit,
        args: ["MyApp", "foo", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    // =========================================================================
    // Limit validation tests
    // =========================================================================

    crate::cli_limit_tests! {
        command: "function",
        variant: Function,
        required_args: ["MyApp", "foo"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
