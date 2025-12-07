//! CLI parsing tests for struct command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "struct",
        test_name: test_requires_module,
        required_arg: "--module",
    }

    crate::cli_option_test! {
        command: "struct",
        variant: Struct,
        test_name: test_with_module,
        args: ["--module", "MyApp.User"],
        field: module,
        expected: "MyApp.User",
    }

    crate::cli_option_test! {
        command: "struct",
        variant: Struct,
        test_name: test_with_regex,
        args: ["--module", "MyApp\\..*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "struct",
        variant: Struct,
        test_name: test_with_project,
        args: ["--module", "MyApp.User", "--project", "my_app"],
        field: project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "struct",
        variant: Struct,
        test_name: test_with_limit,
        args: ["--module", "MyApp.User", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "struct",
        variant: Struct,
        required_args: ["--module", "MyApp.User"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
