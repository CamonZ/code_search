//! CLI parsing tests for depended-by command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "depended-by",
        test_name: test_requires_module,
        required_arg: "<MODULE>",
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_module,
        args: ["MyApp.Repo"],
        field: module,
        expected: "MyApp.Repo",
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_regex,
        args: ["MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_limit,
        args: ["MyApp.Repo", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "depended-by",
        variant: DependedBy,
        required_args: ["MyApp.Repo"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
