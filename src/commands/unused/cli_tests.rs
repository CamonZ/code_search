//! CLI parsing tests for unused command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    // Unused has no required args, so test defaults
    crate::cli_defaults_test! {
        command: "unused",
        variant: Unused,
        required_args: [],
        defaults: {
            common.project: "default",
            common.regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common.limit: 100,
        },
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_module,
        args: ["--module", "MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_project,
        args: ["--project", "my_app"],
        field: common.project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_regex,
        args: ["--module", "MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_private_only,
        args: ["--private-only"],
        field: private_only,
        expected: true,
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_public_only,
        args: ["--public-only"],
        field: public_only,
        expected: true,
    }

    crate::cli_option_test! {
        command: "unused",
        variant: Unused,
        test_name: test_with_exclude_generated,
        args: ["--exclude-generated"],
        field: exclude_generated,
        expected: true,
    }

    crate::cli_limit_tests! {
        command: "unused",
        variant: Unused,
        required_args: [],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Edge case tests (short flags, conflicts)
    // =========================================================================

    #[rstest]
    fn test_with_short_flags() {
        let args = Args::try_parse_from(["code_search", "unused", "-p", "-x"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.private_only);
                assert!(cmd.exclude_generated);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_public_only_short() {
        let args = Args::try_parse_from(["code_search", "unused", "-P"]).unwrap();
        match args.command {
            crate::commands::Command::Unused(cmd) => {
                assert!(cmd.public_only);
            }
            _ => panic!("Expected Unused command"),
        }
    }

    #[rstest]
    fn test_private_and_public_conflict() {
        let result =
            Args::try_parse_from(["code_search", "unused", "--private-only", "--public-only"]);
        assert!(result.is_err());
    }
}
