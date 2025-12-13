//! CLI parsing tests for duplicates command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // Duplicates has no required args
    crate::cli_defaults_test! {
        command: "duplicates",
        variant: Duplicates,
        required_args: [],
        defaults: {
            common.project: "default",
            common.regex: false,
            exact: false,
            common.limit: 100,
        },
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_module,
        args: ["--module", "MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_module_short,
        args: ["-m", "MyApp.User"],
        field: module,
        expected: Some("MyApp.User".to_string()),
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_exact,
        args: ["--exact"],
        field: exact,
        expected: true,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_regex,
        args: ["--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_limit_short,
        args: ["-l", "75"],
        field: common.limit,
        expected: 75,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_project,
        args: ["--project", "my_project"],
        field: common.project,
        expected: "my_project",
    }

    crate::cli_error_test! {
        command: "duplicates",
        test_name: test_limit_zero_rejected,
        args: ["--limit", "0"],
    }

    crate::cli_error_test! {
        command: "duplicates",
        test_name: test_limit_exceeds_max_rejected,
        args: ["--limit", "1001"],
    }
}
