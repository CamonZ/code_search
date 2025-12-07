//! CLI parsing tests for function command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "function",
        test_name: test_requires_module,
        required_arg: "--module",
    }

    crate::cli_required_arg_test! {
        command: "function",
        test_name: test_requires_function,
        required_arg: "--function",
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_module_and_function,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: module,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_function_name,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_arity,
        args: ["--module", "MyApp.Accounts", "--function", "get_user", "--arity", "1"],
        field: arity,
        expected: Some(1),
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_regex,
        args: ["--module", "MyApp.*", "--function", "get_.*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "function",
        variant: Function,
        test_name: test_with_limit,
        args: ["--module", "MyApp", "--function", "foo", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "function",
        variant: Function,
        required_args: ["--module", "MyApp", "--function", "foo"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
