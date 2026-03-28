//! Execute tests for search command.

#[cfg(test)]
mod tests {
    use super::super::{SearchCmd, SearchKind};
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: type_signatures,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // 3 modules in type_signatures: Accounts, Users, Repo
    crate::execute_test! {
        test_name: test_search_modules_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: ".*MyApp.*".to_string(), // Use regex for substring matching
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind, "modules");
            assert_eq!(result.modules.len(), 3);
        },
    }

    // Functions with "user": get_user/1, get_user/2, list_users, create_user = 4
    crate::execute_test! {
        test_name: test_search_functions_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: ".*user.*".to_string(), // Use regex for substring matching
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind, "functions");
            assert_eq!(result.total_functions, Some(4));
        },
    }

    // Functions containing "get": get_user/1, get_user/2, get_by_email, Repo.get = 4
    crate::execute_test! {
        test_name: test_search_functions_specific,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: ".*get.*".to_string(), // Use regex for substring matching
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_functions, Some(4));
        },
    }

    crate::execute_test! {
        test_name: test_search_functions_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^get_user$".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_functions, Some(2));
            // All functions should be named get_user
            for module in &result.function_modules {
                for f in &module.functions {
                    assert_eq!(f.name, "get_user");
                }
            }
        },
    }

    // Modules ending in Accounts or Users
    crate::execute_test! {
        test_name: test_search_modules_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "\\.(Accounts|Users)$".to_string(),
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.modules.len(), 2);
        },
    }

    // Exact module match
    crate::execute_test! {
        test_name: test_search_modules_exact_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "MyApp.Accounts".to_string(),
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].name, "MyApp.Accounts");
        },
    }

    // Exact function match
    crate::execute_test! {
        test_name: test_search_functions_exact_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "get_user".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_functions, Some(2));
            // All functions should be exactly named get_user
            for module in &result.function_modules {
                for f in &module.functions {
                    assert_eq!(f.name, "get_user");
                }
            }
        },
    }

    // Exact match doesn't find partial matches
    crate::execute_no_match_test! {
        test_name: test_search_functions_exact_no_partial,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "user".to_string(), // Won't match get_user, list_users, etc.
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: function_modules,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_search_modules_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "NonExistent".to_string(),
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: modules,
    }

    crate::execute_no_match_test! {
        test_name: test_search_regex_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^xyz".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        empty_field: function_modules,
    }

    // Boundary test: when zero functions match, total_functions must be None (not Some(0)).
    // This kills the mutant that replaces > with >= in SearchResult::from_functions.
    crate::execute_test! {
        test_name: test_search_functions_no_match_total_is_none,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^xyz_nonexistent$".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind, "functions");
            assert!(result.function_modules.is_empty(), "Should have no function modules");
            assert_eq!(result.total_functions, None,
                "total_functions must be None when zero functions match, not Some(0)");
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_search_with_limit,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: ".*user.*".to_string(), // Use regex for substring matching
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 1,
            },
        },
        assertions: |result| {
            // Limit applies to raw results before grouping
            assert_eq!(result.total_functions, Some(1));
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    #[rstest]
    fn test_search_modules_invalid_regex(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::Execute;

        let cmd = SearchCmd {
            pattern: "[invalid".to_string(), // Unclosed bracket
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };

        let result = cmd.execute(&*populated_db);
        assert!(result.is_err(), "Should reject invalid regex pattern");

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid regex pattern"), "Error should mention 'Invalid regex pattern': {}", msg);
        assert!(msg.contains("[invalid"), "Error should show the pattern: {}", msg);
    }

    #[rstest]
    fn test_search_functions_invalid_regex(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::Execute;

        let cmd = SearchCmd {
            pattern: "*invalid".to_string(), // Invalid repetition
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };

        let result = cmd.execute(&*populated_db);
        assert!(result.is_err(), "Should reject invalid regex pattern");

        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid regex pattern"), "Error should mention 'Invalid regex pattern': {}", msg);
        assert!(msg.contains("*invalid"), "Error should show the pattern: {}", msg);
    }

    #[rstest]
    fn test_search_invalid_regex_non_regex_mode_works(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::Execute;

        // Even invalid regex patterns should work in non-regex mode (treated as literals)
        let cmd = SearchCmd {
            pattern: "[invalid".to_string(),
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: false, // Not using regex mode
                limit: 100,
            },
        };

        let result = cmd.execute(&*populated_db);
        assert!(result.is_ok(), "Should accept any pattern in non-regex mode: {:?}", result.err());
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_functions_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = SearchCmd {
            pattern: ".*user.*".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(
            output.contains("Search: .*user.* (functions)"),
            "Table output should contain header, got: {}",
            output
        );
        assert!(
            output.contains("Functions (4)"),
            "Table should contain function count, got: {}",
            output
        );
        assert!(
            output.contains("get_user/1"),
            "Table should show get_user/1, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_modules_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = SearchCmd {
            pattern: "MyApp.Accounts".to_string(),
            kind: SearchKind::Modules,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(
            output.contains("Search: MyApp.Accounts (modules)"),
            "Table output should contain header, got: {}",
            output
        );
        assert!(
            output.contains("Modules (1):"),
            "Table should contain module count, got: {}",
            output
        );
        assert!(
            output.contains("MyApp.Accounts"),
            "Table should show matching module, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_empty_functions_produces_no_results(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = SearchCmd {
            pattern: "^xyz_nonexistent$".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(
            output.contains("Search: ^xyz_nonexistent$ (functions)"),
            "Header should contain queried pattern, got: {}",
            output
        );
        assert!(
            output.contains("No results found."),
            "Empty result should show empty message, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = SearchCmd {
            pattern: ".*user.*".to_string(),
            kind: SearchKind::Functions,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Should produce valid JSON");
        assert_eq!(parsed["pattern"], ".*user.*");
        assert_eq!(parsed["kind"], "functions");
        assert_eq!(parsed["total_functions"], 4);
    }
}
