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
            by_module: false,
            exclude_generated: false,
            common.limit: 100,
        },
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_module,
        args: ["MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
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
        test_name: test_with_by_module,
        args: ["--by-module"],
        field: by_module,
        expected: true,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_exclude_generated,
        args: ["--exclude-generated"],
        field: exclude_generated,
        expected: true,
    }

    crate::cli_option_test! {
        command: "duplicates",
        variant: Duplicates,
        test_name: test_with_regex,
        args: ["MyApp.*", "--regex"],
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
