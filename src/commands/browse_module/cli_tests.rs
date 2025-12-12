//! CLI parsing tests for browse-module command.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use crate::commands::browse_module::DefinitionKind;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    // Positional argument test - browse-module requires a module or file argument
    #[test]
    fn test_requires_module_or_file() {
        let result = Args::try_parse_from(["code_search", "browse-module"]);
        assert!(result.is_err(), "Should require module_or_file positional argument");
    }

    crate::cli_option_test! {
        command: "browse-module",
        variant: BrowseModule,
        test_name: test_with_module_name,
        args: ["MyApp.Accounts"],
        field: module_or_file,
        expected: "MyApp.Accounts",
    }

    crate::cli_option_test! {
        command: "browse-module",
        variant: BrowseModule,
        test_name: test_with_file_path,
        args: ["lib/accounts.ex"],
        field: module_or_file,
        expected: "lib/accounts.ex",
    }

    crate::cli_option_test! {
        command: "browse-module",
        variant: BrowseModule,
        test_name: test_with_regex,
        args: ["MyApp.*", "--regex"],
        field: regex,
        expected: true,
    }

    crate::cli_option_test! {
        command: "browse-module",
        variant: BrowseModule,
        test_name: test_with_name_filter,
        args: ["MyApp.Accounts", "--name", "get_user"],
        field: name,
        expected: Some("get_user".to_string()),
    }

    crate::cli_option_test! {
        command: "browse-module",
        variant: BrowseModule,
        test_name: test_with_limit,
        args: ["MyApp.Accounts", "--limit", "50"],
        field: limit,
        expected: 50,
    }

    crate::cli_limit_tests! {
        command: "browse-module",
        variant: BrowseModule,
        required_args: ["MyApp.Accounts"],
        limit: {
            field: limit,
            default: 100,
            max: 1000,
        },
    }

    // =========================================================================
    // Kind filter tests (manual - enum variant)
    // =========================================================================

    #[rstest]
    #[case("functions", DefinitionKind::Functions)]
    #[case("specs", DefinitionKind::Specs)]
    #[case("types", DefinitionKind::Types)]
    #[case("structs", DefinitionKind::Structs)]
    fn test_kind_filter(#[case] kind_str: &str, #[case] expected: DefinitionKind) {
        let args = Args::try_parse_from([
            "code_search",
            "browse-module",
            "MyApp.Accounts",
            "--kind",
            kind_str,
        ])
        .expect("Failed to parse args");

        if let crate::commands::Command::BrowseModule(cmd) = args.command {
            assert!(cmd.kind.is_some());
            assert!(matches!(cmd.kind.unwrap(), k if std::mem::discriminant(&k) == std::mem::discriminant(&expected)));
        } else {
            panic!("Expected BrowseModule command");
        }
    }

    #[test]
    fn test_kind_filter_default_is_none() {
        let args = Args::try_parse_from(["code_search", "browse-module", "MyApp.Accounts"])
            .expect("Failed to parse args");

        if let crate::commands::Command::BrowseModule(cmd) = args.command {
            assert!(cmd.kind.is_none());
        } else {
            panic!("Expected BrowseModule command");
        }
    }
}
