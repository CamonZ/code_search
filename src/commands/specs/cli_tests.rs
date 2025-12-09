//! CLI parsing tests for specs command.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "specs",
        test_name: test_requires_module,
        required_arg: "MODULE",
    }

    crate::cli_option_test! {
        command: "specs",
        variant: Specs,
        test_name: test_with_module_only,
        args: ["MyApp.Module"],
        field: module,
        expected: "MyApp.Module",
    }

    crate::cli_option_test! {
        command: "specs",
        variant: Specs,
        test_name: test_with_function,
        args: ["MyApp.Module", "--function", "get_user"],
        field: function,
        expected: Some("get_user".to_string()),
    }

    crate::cli_option_test! {
        command: "specs",
        variant: Specs,
        test_name: test_with_kind,
        args: ["MyApp.Module", "--kind", "callback"],
        field: kind,
        expected: Some("callback".to_string()),
    }

    crate::cli_option_test! {
        command: "specs",
        variant: Specs,
        test_name: test_with_regex,
        args: ["MyApp.*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "specs",
        variant: Specs,
        test_name: test_with_project,
        args: ["MyApp.Module", "--project", "my_app"],
        field: project,
        expected: "my_app",
    }

    crate::cli_limit_tests! {
        command: "specs",
        variant: Specs,
        required_args: ["MyApp.Module"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }
}
