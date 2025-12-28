//! Execute tests for unused command.

/// CozoDB tests use JSON-based fixtures
#[cfg(all(test, not(feature = "backend-surrealdb")))]
mod tests {
    use super::super::UnusedCmd;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Uncalled functions: index, show, create (Controller), get_user/2 + validate_email (Accounts), insert (Repo) = 6
    // Note: get_user/1 is called but get_user/2 is not (Controller.show calls arity 1 only)
    crate::execute_test! {
        test_name: test_unused_finds_uncalled_functions,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 6);
            let all_funcs: Vec<&str> = result.items.iter()
                .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
                .collect();
            assert!(all_funcs.contains(&"validate_email"));
            assert!(all_funcs.contains(&"insert"));
        },
    }

    // In Accounts: validate_email (defp) and get_user/2 (def, not called) = 2
    crate::execute_test! {
        test_name: test_unused_with_module_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some(".*Accounts.*".to_string()), // Use regex for substring matching
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2);
        },
    }

    // Controller has 3 uncalled functions
    crate::execute_test! {
        test_name: test_unused_with_regex_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3);
        },
    }

    // Exact module match - MyApp.Accounts has 2 uncalled functions
    crate::execute_test! {
        test_name: test_unused_exact_module_match,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("MyApp.Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2);
            // Verify all results are from MyApp.Accounts
            for module_group in &result.items {
                assert_eq!(module_group.name, "MyApp.Accounts");
            }
        },
    }

    // Exact match doesn't find partial matches
    crate::execute_no_match_test! {
        test_name: test_unused_exact_no_partial,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("Accounts".to_string()), // Won't match "MyApp.Accounts"
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_unused_no_match,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("NonExistent".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_unused_with_limit,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 1,
            },
        },
        assertions: |result| {
            // Limit applies to raw results before grouping
            assert_eq!(result.total_items, 1);
        },
    }

    // validate_email is the only private (defp) uncalled function
    crate::execute_test! {
        test_name: test_unused_private_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 1);
            assert_eq!(result.items[0].entries[0].name, "validate_email");
            assert_eq!(result.items[0].entries[0].kind, "defp");
        },
    }

    // 5 public uncalled: index, show, create (Controller), get_user/2 (Accounts), insert (Repo)
    crate::execute_test! {
        test_name: test_unused_public_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 5);
            for module in &result.items {
                for func in &module.entries {
                    assert_eq!(func.kind, "def");
                }
            }
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: UnusedCmd,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}

/// SurrealDB tests use programmatically created fixtures
#[cfg(all(test, feature = "backend-surrealdb"))]
mod tests_surrealdb {
    use super::super::UnusedCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::collections::HashSet;

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // The SurrealDB complex fixture has 16 unused functions:
    // - 3 private: validate_email, debug, transform_data
    // - 13 public: __struct__, __generated__ x2, format_name, format_display,
    //              fetch, create, index, show, subscribe, increment, validate x2
    #[rstest]
    fn test_unused_finds_uncalled_functions(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 16, "Should find 16 unused functions");

        let all_funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();

        assert!(all_funcs.contains("validate_email"));
        assert!(all_funcs.contains("transform_data"));
        assert!(all_funcs.contains("index"));
        assert!(all_funcs.contains("show"));
    }

    // Accounts has 4 unused: __generated__, __struct__, format_name, validate_email
    #[rstest]
    fn test_unused_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("MyApp.Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 4,
            "Accounts should have 4 unused functions"
        );

        // Verify all results are from MyApp.Accounts
        for module_group in &result.items {
            assert_eq!(module_group.name, "MyApp.Accounts");
        }

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("__generated__"));
        assert!(funcs.contains("__struct__"));
        assert!(funcs.contains("format_name"));
        assert!(funcs.contains("validate_email"));
    }

    // Controller has 5 unused: __generated__, create, format_display, index, show
    #[rstest]
    fn test_unused_with_regex_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 5,
            "Controller should have 5 unused functions"
        );

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("__generated__"));
        assert!(funcs.contains("create"));
        assert!(funcs.contains("format_display"));
        assert!(funcs.contains("index"));
        assert!(funcs.contains("show"));
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_unused_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("NonExistent".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(result.items.is_empty(), "Expected no results for non-existent module");
        assert_eq!(result.total_items, 0);
    }

    #[rstest]
    fn test_unused_exact_no_partial(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("Accounts".to_string()), // Won't match "MyApp.Accounts"
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Partial match 'Accounts' should not match 'MyApp.Accounts'"
        );
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_unused_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 3,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 3, "Limit should restrict to 3 results");
    }

    // 3 private unused: validate_email, debug, transform_data
    #[rstest]
    fn test_unused_private_only(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 3,
            "Should find 3 private unused functions"
        );

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("validate_email"));
        assert!(funcs.contains("debug"));
        assert!(funcs.contains("transform_data"));

        // All should be private (defp or defmacrop)
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    func.kind == "defp" || func.kind == "defmacrop",
                    "Expected private function, got {} for {}",
                    func.kind,
                    func.name
                );
            }
        }
    }

    // 13 public unused
    #[rstest]
    fn test_unused_public_only(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 13,
            "Should find 13 public unused functions"
        );

        // All should be public (def or defmacro)
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    func.kind == "def" || func.kind == "defmacro",
                    "Expected public function, got {} for {}",
                    func.kind,
                    func.name
                );
            }
        }
    }

    // Excluding generated should reduce from 16 to 13
    #[rstest]
    fn test_unused_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 13,
            "Excluding generated should leave 13 functions (16 - 3 generated)"
        );

        // Verify no generated functions
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    !func.name.starts_with("__"),
                    "Should not contain generated function: {}",
                    func.name
                );
            }
        }
    }

    // Combined: public + exclude_generated = 10 (13 public - 3 generated)
    #[rstest]
    fn test_unused_public_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 10,
            "Public + exclude_generated should leave 10 functions"
        );
    }

    // Notifier has no unused functions (all are called)
    #[rstest]
    fn test_unused_notifier_empty(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("MyApp.Notifier".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(
            result.items.is_empty(),
            "Notifier should have no unused functions"
        );
        assert_eq!(result.total_items, 0);
    }
}
