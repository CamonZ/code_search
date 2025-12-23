//! CLI parsing tests for calls-to command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "calls-to",
        test_name: test_requires_module,
        required_arg: "<MODULE>",
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_module,
        args: ["MyApp.Repo"],
        field: module,
        expected: "MyApp.Repo",
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_function,
        args: ["MyApp.Repo", "get"],
        field: function,
        expected: Some("get".to_string()),
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_arity,
        args: ["MyApp.Repo", "get", "2"],
        field: arity,
        expected: Some(2),
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_regex,
        args: ["MyApp\\.Repo", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_limit,
        args: ["MyApp.Repo", "--limit", "25"],
        field: common.limit,
        expected: 25,
    }

    crate::cli_limit_tests! {
        command: "calls-to",
        variant: CallsTo,
        required_args: ["MyApp.Repo"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
