//! CLI parsing tests for file command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "file",
        test_name: test_requires_file,
        required_arg: "--file",
    }

    crate::cli_option_test! {
        command: "file",
        variant: File,
        test_name: test_with_file,
        args: ["--file", "lib/accounts.ex"],
        field: file,
        expected: "lib/accounts.ex",
    }

    crate::cli_option_test! {
        command: "file",
        variant: File,
        test_name: test_with_project,
        args: ["--file", "lib/accounts.ex", "--project", "my_app"],
        field: project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "file",
        variant: File,
        test_name: test_with_regex,
        args: ["--file", "lib/.*\\.ex$", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "file",
        variant: File,
        test_name: test_with_limit,
        args: ["--file", "lib/accounts.ex", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "file",
        variant: File,
        required_args: ["--file", "lib/accounts.ex"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
