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
        required_arg: "--module",
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_module,
        args: ["--module", "MyApp.Repo"],
        field: module,
        expected: "MyApp.Repo",
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_function,
        args: ["--module", "MyApp.Repo", "--function", "get"],
        field: function,
        expected: Some("get".to_string()),
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_arity,
        args: ["--module", "MyApp.Repo", "--function", "get", "--arity", "2"],
        field: arity,
        expected: Some(2),
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_regex,
        args: ["--module", "MyApp\\.Repo", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "calls-to",
        variant: CallsTo,
        test_name: test_with_limit,
        args: ["--module", "MyApp.Repo", "--limit", "25"],
        field: limit,
        expected: 25,
    }

    crate::cli_limit_tests! {
        command: "calls-to",
        variant: CallsTo,
        required_args: ["--module", "MyApp.Repo"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
