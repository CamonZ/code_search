//! CLI parsing tests for accepts command.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "accepts",
        test_name: test_requires_pattern,
        required_arg: "<PATTERN>",
    }

    crate::cli_option_test! {
        command: "accepts",
        variant: Accepts,
        test_name: test_with_pattern,
        args: ["User.t"],
        field: pattern,
        expected: "User.t",
    }

    crate::cli_option_test! {
        command: "accepts",
        variant: Accepts,
        test_name: test_with_module_filter,
        args: ["User.t", "MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
    }

    crate::cli_option_test! {
        command: "accepts",
        variant: Accepts,
        test_name: test_with_regex,
        args: ["User.*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "accepts",
        variant: Accepts,
        test_name: test_with_limit,
        args: ["User.t", "--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "accepts",
        variant: Accepts,
        required_args: ["User.t"],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Default value tests
    // =========================================================================

    crate::cli_defaults_test! {
        command: "accepts",
        variant: Accepts,
        required_args: ["User.t"],
        defaults: {
            pattern: "User.t",
            module: None::<String>,
            common.regex: false,
            common.limit: 100,
        },
    }
}
