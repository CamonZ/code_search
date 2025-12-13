//! CLI parsing tests for hotspots command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::commands::hotspots::HotspotKind;
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    // Test default values
    crate::cli_defaults_test! {
        command: "hotspots",
        variant: Hotspots,
        required_args: [],
        defaults: {
            common.project: "default",
            common.regex: false,
            common.limit: 100,
        },
    }

    // Test individual options
    crate::cli_option_test! {
        command: "hotspots",
        variant: Hotspots,
        test_name: test_with_module,
        args: ["--module", "MyApp"],
        field: module,
        expected: Some("MyApp".to_string()),
    }

    crate::cli_option_test! {
        command: "hotspots",
        variant: Hotspots,
        test_name: test_with_project,
        args: ["--project", "my_app"],
        field: common.project,
        expected: "my_app",
    }

    crate::cli_option_test! {
        command: "hotspots",
        variant: Hotspots,
        test_name: test_with_regex,
        args: ["--module", "MyApp\\..*", "--regex"],
        field: common.regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "hotspots",
        variant: Hotspots,
        test_name: test_with_limit,
        args: ["--limit", "50"],
        field: common.limit,
        expected: 50,
    }

    // Test limit validation
    crate::cli_limit_tests! {
        command: "hotspots",
        variant: Hotspots,
        required_args: [],
        limit: {
            field: common.limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Edge case tests (require matches! macro for enum variant matching)
    // =========================================================================

    #[rstest]
    fn test_kind_default_is_incoming() {
        let args = Args::try_parse_from(["code_search", "hotspots"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Incoming));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_kind_outgoing() {
        let args =
            Args::try_parse_from(["code_search", "hotspots", "--kind", "outgoing"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Outgoing));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }

    #[rstest]
    fn test_kind_total() {
        let args = Args::try_parse_from(["code_search", "hotspots", "--kind", "total"]).unwrap();
        match args.command {
            crate::commands::Command::Hotspots(cmd) => {
                assert!(matches!(cmd.kind, HotspotKind::Total));
            }
            _ => panic!("Expected Hotspots command"),
        }
    }
}
