//! CLI parsing tests for calls-from command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "calls-from",
        test_name: test_requires_module,
        required_arg: "--module",
    }

    crate::cli_option_test! {
        command: "calls-from",
        variant: CallsFrom,
        test_name: test_with_module,
        args: ["--module", "MyApp.Accounts"],
        field: module,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "calls-from",
        variant: CallsFrom,
        test_name: test_with_function,
        args: ["--module", "MyApp.Accounts", "--function", "get_user"],
        field: function,
        expected: Some("get_user".to_string()),
    }

    crate::cli_option_test! {
        command: "calls-from",
        variant: CallsFrom,
        test_name: test_with_arity,
        args: ["--module", "MyApp.Accounts", "--function", "get_user", "--arity", "1"],
        field: arity,
        expected: Some(1),
    }

    crate::cli_option_test! {
        command: "calls-from",
        variant: CallsFrom,
        test_name: test_with_regex,
        args: ["--module", "MyApp.*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "calls-from",
        variant: CallsFrom,
        test_name: test_with_limit,
        args: ["--module", "MyApp.Accounts", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "calls-from",
        variant: CallsFrom,
        required_args: ["--module", "MyApp.Accounts"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
