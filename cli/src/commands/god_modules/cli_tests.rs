//! CLI parsing tests for god_modules command.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    // Test default values
    crate::cli_defaults_test! {
        command: "god-modules",
        variant: GodModules,
        required_args: [],
        defaults: {
            common.regex: false,
            common.limit: 100,
            min_functions: 20,
            min_loc: 0,
            min_total: 10,
            module: None::<String>,
        },
    }

    // Test positional module argument
    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_module,
        args: ["MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_regex,
        args: ["MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_limit_short,
        args: ["-l", "75"],
        field: common.limit,
        expected: 75,
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_min_functions,
        args: ["--min-functions", "30"],
        field: min_functions,
        expected: 30,
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_min_loc,
        args: ["--min-loc", "500"],
        field: min_loc,
        expected: 500,
    }

    crate::cli_option_test! {
        command: "god-modules",
        variant: GodModules,
        test_name: test_with_min_total,
        args: ["--min-total", "15"],
        field: min_total,
        expected: 15,
    }

    // Test limit validation
    crate::cli_limit_tests! {
        command: "god-modules",
        variant: GodModules,
        required_args: [],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

}
