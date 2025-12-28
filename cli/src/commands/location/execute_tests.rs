//! Execute tests for location command.

/// CozoDB tests use JSON-based fixtures
#[cfg(all(test, not(feature = "backend-surrealdb")))]
mod tests {
    use super::super::LocationCmd;
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

    crate::execute_test! {
        test_name: test_location_exact_match,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].functions.len(), 1);
            let func = &result.modules[0].functions[0];
            assert_eq!(func.file, "lib/my_app/accounts.ex");
            assert_eq!(func.clauses[0].start_line, 10);
            assert_eq!(func.clauses[0].end_line, 15);
        },
    }

    // get_user exists in Accounts with arities 1 and 2
    crate::execute_test! {
        test_name: test_location_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // 2 functions (get_user/1 and get_user/2) in 1 module
            assert_eq!(result.total_clauses, 2);
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].name, "MyApp.Accounts");
            assert_eq!(result.modules[0].functions.len(), 2);
        },
    }

    // Functions with "user" in name: get_user/1, get_user/2, list_users = 3
    crate::execute_test! {
        test_name: test_location_without_module_multiple_matches,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 3);
        },
    }

    // get_user has two arities in Accounts
    crate::execute_test! {
        test_name: test_location_without_arity,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 2);
        },
    }

    crate::execute_test! {
        test_name: test_location_with_regex,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp\\..*".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 3);
        },
    }

    crate::execute_test! {
        test_name: test_location_format,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            let func = &result.modules[0].functions[0];
            assert_eq!(
                format!("{}:{}:{}", func.file, func.clauses[0].start_line, func.clauses[0].end_line),
                "lib/my_app/accounts.ex:10:15"
            );
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_location_no_match,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("NonExistent".to_string()),
            function: "foo".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: modules,
    }

    #[cfg(not(feature = "backend-surrealdb"))]
    crate::execute_no_match_test! {
        test_name: test_location_nonexistent_project,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "nonexistent_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: modules,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[cfg(not(feature = "backend-surrealdb"))]
    crate::execute_test! {
        test_name: test_location_with_project_filter,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].functions.len(), 1);
        },
    }

    // 6 functions with arity 1: get_user/1, validate_email, process, fetch, all, notify
    crate::execute_test! {
        test_name: test_location_arity_filter_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            let total_funcs: usize = result.modules.iter().map(|m| m.functions.len()).sum();
            assert_eq!(total_funcs, 6);
            // All functions should have arity 1
            for module in &result.modules {
                for func in &module.functions {
                    assert_eq!(func.arity, 1);
                }
            }
        },
    }

    #[cfg(not(feature = "backend-surrealdb"))]
    crate::execute_test! {
        test_name: test_location_project_filter_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 2);
        },
    }

    // Accounts has get_user/1, get_user/2, list_users matching ".*user.*" = 3
    crate::execute_test! {
        test_name: test_location_function_regex_with_exact_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 3);
        },
    }

    crate::execute_test! {
        test_name: test_location_arity_zero,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "list_users".to_string(),
            arity: Some(0),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_clauses, 1);
            assert_eq!(result.modules[0].functions[0].arity, 0);
        },
    }

    crate::execute_test! {
        test_name: test_location_with_limit,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 1,
            },
        },
        assertions: |result| {
            // Limit applies to raw results before grouping
            assert_eq!(result.total_clauses, 1);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: LocationCmd,
        cmd: LocationCmd {
            module: Some("MyApp".to_string()),
            function: "foo".to_string(),
            arity: None,
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
    use super::super::LocationCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // The complex fixture has clauses:
    // - Accounts: get_user/1 at line 10, get_user/2 at line 18, list_users/0 at line 24,
    //             notify_change/1 at line 40, validate_email/1 at line 30, format_name/1 at line 36,
    //             __struct__/0 at line 1, __generated__/0 at line 45
    // - Controller: index/2 at line 5, show/2 at line 12, create/2 at lines 25 and 28,
    //               handle_event/1 at line 35, format_display/1 at line 42, __generated__/0 at line 50
    // - Service: process_request/2 at lines 8, 12, 16 (3 clauses), transform_data/1 at line 22,
    //            get_context/1 at line 28, validate/1 at line 32
    // - Repo: get/2 at line 10, all/1 at line 15, insert/1 at line 20, query/2 at line 28,
    //         validate/1 at line 35

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_location_exact_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "MyApp.Accounts");
        assert_eq!(result.modules[0].functions.len(), 1);

        let func = &result.modules[0].functions[0];
        assert_eq!(func.name, "get_user");
        assert_eq!(func.arity, 1);
        assert_eq!(func.file, "lib/my_app/accounts.ex");
        assert_eq!(func.clauses[0].start_line, 10);
    }

    // get_user/1 has 2 clauses (lines 10, 12), get_user/2 has 1 clause (line 17) = 3 total
    #[rstest]
    fn test_location_without_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 3, "get_user/1 has 2 clauses + get_user/2 has 1");
        assert_eq!(result.modules[0].functions.len(), 2, "Two function arities");
    }

    // process_request/2 has 3 clauses at lines 8, 12, 16
    #[rstest]
    fn test_location_multiple_clauses(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Service".to_string()),
            function: "process_request".to_string(),
            arity: Some(2),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].functions.len(), 1);

        let func = &result.modules[0].functions[0];
        assert_eq!(func.clauses.len(), 3, "process_request/2 has 3 clauses");

        // Verify clause lines
        let lines: Vec<i64> = func.clauses.iter().map(|c| c.start_line).collect();
        assert!(lines.contains(&8), "Should have clause at line 8");
        assert!(lines.contains(&12), "Should have clause at line 12");
        assert!(lines.contains(&16), "Should have clause at line 16");
    }

    #[rstest]
    fn test_location_without_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1 has 2 clauses, get_user/2 has 1 clause = 3 total
        assert_eq!(result.total_clauses, 3);
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "MyApp.Accounts");
    }

    // =========================================================================
    // Regex tests
    // =========================================================================

    #[rstest]
    fn test_location_with_regex(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp\\..*".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1 (2 clauses) + get_user/2 (1 clause) + list_users/0 (1 clause) = 4
        assert_eq!(result.total_clauses, 4);
    }

    // validate exists in multiple modules (Service, Repo)
    #[rstest]
    fn test_location_function_across_modules(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "validate".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // validate/1 exists in both Service and Repo
        assert_eq!(result.total_clauses, 2, "validate/1 in Service and Repo");
        assert_eq!(result.modules.len(), 2, "Should be in 2 modules");
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_location_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("NonExistent".to_string()),
            function: "foo".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.modules.is_empty());
        assert_eq!(result.total_clauses, 0);
    }

    #[rstest]
    fn test_location_wrong_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(99), // Non-existent arity
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.modules.is_empty());
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_location_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: ".*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 3,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 3, "Limit should restrict to 3 clauses");
    }

    #[rstest]
    fn test_location_arity_zero(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "list_users".to_string(),
            arity: Some(0),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 1);
        assert_eq!(result.modules[0].functions[0].arity, 0);
    }

    // =========================================================================
    // Output format tests
    // =========================================================================

    #[rstest]
    fn test_location_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(!result.modules.is_empty(), "Should find at least one module");
        let func = &result.modules[0].functions[0];
        // Verify we can construct a file:line format
        assert!(func.file.ends_with(".ex"), "File should be .ex: {}", func.file);
        assert!(func.clauses[0].start_line > 0);
    }
}
