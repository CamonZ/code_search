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
        required_arg: "--module",
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_module,
        args: ["--module", "MyApp.Repo"],
        field: module,
        expected: "MyApp.Repo",
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_regex,
        args: ["--module", "MyApp\\..*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "depended-by",
        variant: DependedBy,
        test_name: test_with_limit,
        args: ["--module", "MyApp.Repo", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "depended-by",
        variant: DependedBy,
        required_args: ["--module", "MyApp.Repo"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
