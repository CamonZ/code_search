//! CLI parsing tests for location command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "location",
        test_name: test_requires_function,
        required_arg: "--function",
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_function_only,
        args: ["--function", "get_user"],
        field: function,
        expected: "get_user",
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_module,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: module,
        expected: Some("MyApp.Accounts".to_string()),
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_arity,
        args: ["--module", "MyApp.Accounts", "--function", "get_user", "--arity", "1"],
        field: arity,
        expected: Some(1),
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_regex,
        args: ["--module", "MyApp.*", "--function", "get_.*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_project,
        args: ["--module", "MyApp.Accounts", "--function", "get_user", "--project", "my_app"],
        field: project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "location",
        variant: Location,
        test_name: test_with_limit,
        args: ["--function", "get_user", "--limit", "10"],
        field: limit,
        expected: 10,
    }

    crate::cli_limit_tests! {
        command: "location",
        variant: Location,
        required_args: ["--function", "get_user"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
