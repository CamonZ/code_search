//! CLI parsing tests for struct-usage command.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Required argument tests
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "struct-usage",
        test_name: test_requires_pattern,
        required_arg: "<PATTERN>",
    }

    // =========================================================================
    // Option tests
    // =========================================================================

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_with_pattern,
        args: ["User.t"],
        field: pattern,
        expected: "User.t",
    }

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_with_module,
        args: ["User.t", "MyApp.Accounts"],
        field: module,
        expected: Some("MyApp.Accounts".to_string()),
    }

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_with_by_module,
        args: ["User.t", "--by-module"],
        field: by_module,
        expected: true,
    }

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_by_module_default_false,
        args: ["User.t"],
        field: by_module,
        expected: false,
    }

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_with_regex,
        args: [".*\\.t", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "struct-usage",
        variant: StructUsage,
        test_name: test_with_limit,
        args: ["User.t", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    // =========================================================================
    // Limit validation tests
    // =========================================================================

    crate::cli_limit_tests! {
        command: "struct-usage",
        variant: StructUsage,
        required_args: ["User.t"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }
}
