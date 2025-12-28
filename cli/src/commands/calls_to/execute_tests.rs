//! Execute tests for calls-to command.

/// CozoDB tests use JSON-based fixtures
#[cfg(all(test, not(feature = "backend-surrealdb")))]
mod tests {
    use super::super::CallsToCmd;
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

    // 4 calls to MyApp.Repo: get_user/1→get, get_user/2→get, list_users→all, do_fetch→get
    crate::execute_test! {
        test_name: test_calls_to_module,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 4,
                "Expected 4 total calls to MyApp.Repo");
        },
    }

    // 3 calls to Repo.get: from get_user/1, get_user/2, do_fetch
    crate::execute_test! {
        test_name: test_calls_to_function,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3,
                "Expected 3 calls to MyApp.Repo.get");
        },
    }

    crate::execute_test! {
        test_name: test_calls_to_function_with_arity,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: Some(2),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3);
            // All callee functions should be get/2
            for module in &result.items {
                for func in &module.entries {
                    assert_eq!(func.arity, 2);
                }
            }
        },
    }

    // 4 calls match get|all: 3 to get + 1 to all
    crate::execute_test! {
        test_name: test_calls_to_regex_function,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get|all".to_string()),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 4,
                "Expected 4 calls to get|all");
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_calls_to_no_match,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(result.items.is_empty(), "Expected no modules for non-existent target");
            assert_eq!(result.total_items, 0);
        },
    }

    crate::execute_test! {
        test_name: test_calls_to_nonexistent_arity,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: Some(99),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(result.items.is_empty(), "Expected no results for non-existent arity");
            assert_eq!(result.total_items, 0);
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[cfg(not(feature = "backend-surrealdb"))]
    crate::execute_test! {
        test_name: test_calls_to_with_project_filter,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(result.total_items > 0, "Should have calls with project filter");
        },
    }

    crate::execute_test! {
        test_name: test_calls_to_with_limit,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2, "Limit should restrict to 2 calls");
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: CallsToCmd,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
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
    use super::super::CallsToCmd;
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

    // 5 calls TO MyApp.Repo in the complex fixture:
    // - Accounts.get_user/1 → Repo.get/2
    // - Accounts.list_users/0 → Repo.all/1
    // - Repo.get/2 → Repo.query/2
    // - Repo.all/1 → Repo.query/2
    // - Logger.log_query/2 → Repo.insert/1
    #[rstest]
    fn test_calls_to_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 5, "Expected 5 calls TO MyApp.Repo");

        // Collect all calls as (caller_module, caller_name, caller_arity, callee_name, callee_arity)
        let mut actual_calls: HashSet<(String, String, i64, String, i64)> = HashSet::new();
        for module_group in &result.items {
            for func in &module_group.entries {
                for call in &func.callers {
                    actual_calls.insert((
                        call.caller.module.to_string(),
                        call.caller.name.to_string(),
                        call.caller.arity,
                        func.name.clone(),
                        func.arity,
                    ));
                }
            }
        }

        // Verify expected calls
        assert!(
            actual_calls.contains(&("MyApp.Accounts".to_string(), "get_user".to_string(), 1, "get".to_string(), 2)),
            "Should contain Accounts.get_user/1 → Repo.get/2"
        );
        assert!(
            actual_calls.contains(&("MyApp.Accounts".to_string(), "list_users".to_string(), 0, "all".to_string(), 1)),
            "Should contain Accounts.list_users/0 → Repo.all/1"
        );
        assert!(
            actual_calls.contains(&("MyApp.Repo".to_string(), "get".to_string(), 2, "query".to_string(), 2)),
            "Should contain Repo.get/2 → Repo.query/2"
        );
        assert!(
            actual_calls.contains(&("MyApp.Repo".to_string(), "all".to_string(), 1, "query".to_string(), 2)),
            "Should contain Repo.all/1 → Repo.query/2"
        );
        assert!(
            actual_calls.contains(&("MyApp.Logger".to_string(), "log_query".to_string(), 2, "insert".to_string(), 1)),
            "Should contain Logger.log_query/2 → Repo.insert/1"
        );
    }

    // 1 call TO Repo.get: from Accounts.get_user/1
    #[rstest]
    fn test_calls_to_function(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1, "Expected 1 call TO MyApp.Repo.get");

        // Verify the caller
        let module_group = &result.items[0];
        let func = &module_group.entries[0];
        assert_eq!(func.name, "get");
        assert_eq!(func.arity, 2);
        assert_eq!(func.callers.len(), 1);
        assert_eq!(func.callers[0].caller.module.as_ref(), "MyApp.Accounts");
        assert_eq!(func.callers[0].caller.name.as_ref(), "get_user");
        assert_eq!(func.callers[0].caller.arity, 1);
    }

    #[rstest]
    fn test_calls_to_function_with_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: Some(2),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        // All callee functions should be get/2
        for module in &result.items {
            for func in &module.entries {
                assert_eq!(func.arity, 2);
                assert_eq!(func.name, "get");
            }
        }
    }

    // 2 calls match get|all: Accounts.get_user/1→get/2 + Accounts.list_users/0→all/1
    #[rstest]
    fn test_calls_to_regex_function(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get|all".to_string()),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 2, "Expected 2 calls TO get|all");
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_calls_to_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no modules for non-existent target"
        );
        assert_eq!(result.total_items, 0);
    }

    #[rstest]
    fn test_calls_to_nonexistent_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: Some(99),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no results for non-existent arity"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_calls_to_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 2, "Limit should restrict to 2 calls");
    }
}
