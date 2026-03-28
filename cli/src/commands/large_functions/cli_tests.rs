//! CLI parsing tests for large_functions command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_defaults_test! {
        command: "large-functions",
        variant: LargeFunctions,
        required_args: [],
        defaults: {
            min_lines: 50,
            include_generated: false,
            module: None,
            common.regex: false,
            common.limit: 100,
        },
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_min_lines,
        args: ["--min-lines", "100"],
        field: min_lines,
        expected: 100,
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_include_generated,
        args: ["--include-generated"],
        field: include_generated,
        expected: true,
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_module,
        args: ["MyApp.Accounts"],
        field: module,
        expected: Some("MyApp.Accounts".to_string()),
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_regex,
        args: ["MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_with_limit_short,
        args: ["-l", "20"],
        field: common.limit,
        expected: 20,
    }

    crate::cli_limit_tests! {
        command: "large-functions",
        variant: LargeFunctions,
        required_args: [],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    crate::cli_option_test! {
        command: "large-functions",
        variant: LargeFunctions,
        test_name: test_combined_options,
        args: ["MyApp", "--min-lines", "100", "--include-generated", "-l", "30"],
        field: min_lines,
        expected: 100,
    }
}
