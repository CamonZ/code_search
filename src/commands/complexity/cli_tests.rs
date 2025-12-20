//! CLI parsing tests for complexity command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_defaults_test! {
        command: "complexity",
        variant: Complexity,
        required_args: [],
        defaults: {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: None,
            common.project: "default".to_string(),
            common.regex: false,
            common.limit: 100,
        },
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_min,
        args: ["--min", "10"],
        field: min,
        expected: 10,
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_min_depth,
        args: ["--min-depth", "3"],
        field: min_depth,
        expected: 3,
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_exclude_generated,
        args: ["--exclude-generated"],
        field: exclude_generated,
        expected: true,
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_module,
        args: ["MyApp.Accounts"],
        field: module,
        expected: Some("MyApp.Accounts".to_string()),
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_project,
        args: ["--project", "my_project"],
        field: common.project,
        expected: "my_project".to_string(),
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_regex,
        args: ["MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_with_limit_short,
        args: ["-l", "20"],
        field: common.limit,
        expected: 20,
    }

    crate::cli_limit_tests! {
        command: "complexity",
        variant: Complexity,
        required_args: [],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    crate::cli_option_test! {
        command: "complexity",
        variant: Complexity,
        test_name: test_combined_options,
        args: ["MyApp", "--min", "15", "--min-depth", "2", "--exclude-generated", "-l", "30"],
        field: min,
        expected: 15,
    }
}
