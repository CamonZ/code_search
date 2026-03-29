//! Execute tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::CallsFromCmd;
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

    // MyApp.Accounts has 4 calls in the complex fixture:
    // - get_user/1 → MyApp.Repo.get/2
    // - get_user/2 → MyApp.Accounts.get_user/1
    // - list_users/0 → MyApp.Repo.all/1
    // - notify_change/1 → MyApp.Controller.handle_event/1
    #[rstest]
    fn test_calls_from_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 4, "Expected 4 calls from MyApp.Accounts");

        // Collect all calls as (caller_name, caller_arity, callee_module, callee_name, callee_arity)
        let mut actual_calls: HashSet<(String, i64, String, String, i64)> = HashSet::new();
        for module_group in &result.items {
            for func in &module_group.entries {
                for call in &func.calls {
                    actual_calls.insert((
                        func.name.clone(),
                        func.arity,
                        call.callee.module.to_string(),
                        call.callee.name.to_string(),
                        call.callee.arity,
                    ));
                }
            }
        }

        // Verify expected calls
        assert!(
            actual_calls.contains(&("get_user".to_string(), 1, "MyApp.Repo".to_string(), "get".to_string(), 2)),
            "Should contain get_user/1 → Repo.get/2"
        );
        assert!(
            actual_calls.contains(&("get_user".to_string(), 2, "MyApp.Accounts".to_string(), "get_user".to_string(), 1)),
            "Should contain get_user/2 → get_user/1"
        );
        assert!(
            actual_calls.contains(&("list_users".to_string(), 0, "MyApp.Repo".to_string(), "all".to_string(), 1)),
            "Should contain list_users/0 → Repo.all/1"
        );
        assert!(
            actual_calls.contains(&("notify_change".to_string(), 1, "MyApp.Controller".to_string(), "handle_event".to_string(), 1)),
            "Should contain notify_change/1 → Controller.handle_event/1"
        );
    }

    // get_user functions: get_user/1→Repo.get, get_user/2→get_user/1 = 2 calls
    #[rstest]
    fn test_calls_from_function(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 2, "Expected 2 calls from get_user functions");

        // Verify both get_user variants are present
        let mut found_get_user_1 = false;
        let mut found_get_user_2 = false;
        for module_group in &result.items {
            for func in &module_group.entries {
                assert_eq!(func.name, "get_user", "All functions should be get_user");
                if func.arity == 1 {
                    found_get_user_1 = true;
                    assert_eq!(func.calls.len(), 1);
                    assert_eq!(func.calls[0].callee.module.as_ref(), "MyApp.Repo");
                    assert_eq!(func.calls[0].callee.name.as_ref(), "get");
                } else if func.arity == 2 {
                    found_get_user_2 = true;
                    assert_eq!(func.calls.len(), 1);
                    assert_eq!(func.calls[0].callee.module.as_ref(), "MyApp.Accounts");
                    assert_eq!(func.calls[0].callee.name.as_ref(), "get_user");
                }
            }
        }
        assert!(found_get_user_1, "Should find get_user/1");
        assert!(found_get_user_2, "Should find get_user/2");
    }

    // All calls from MyApp.* modules - there are 24 calls in the complex fixture
    #[rstest]
    fn test_calls_from_regex_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // The complex fixture has 24 calls total from MyApp.* modules
        assert_eq!(result.total_items, 24, "Expected 24 calls from MyApp.* modules");

        // Verify we have calls from multiple modules
        let modules: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(modules.contains("MyApp.Accounts"), "Should include MyApp.Accounts");
        assert!(modules.contains("MyApp.Controller"), "Should include MyApp.Controller");
        assert!(modules.contains("MyApp.Service"), "Should include MyApp.Service");
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_calls_from_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no modules for non-existent module"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // Direction verification tests
    // =========================================================================

    /// Verify that calls_from returns outgoing calls (caller → callee direction).
    ///
    /// If CallDirection::From were swapped to CallDirection::To, this test would fail
    /// because the result would contain 5 incoming calls to Repo rather than 3 outgoing.
    #[rstest]
    fn test_calls_from_returns_outgoing_direction(populated_db: Box<dyn db::backend::Database>) {
        // MyApp.Repo makes 3 outgoing calls:
        // - Repo.get/2 → Repo.query/2
        // - Repo.all/1 → Repo.query/2
        // - Repo.insert/1 → Service.get_context/1
        //
        // Calls TO MyApp.Repo would be 5 (a different count):
        // - Accounts.get_user/1 → Repo.get/2
        // - Accounts.list_users/0 → Repo.all/1
        // - Repo.get/2 → Repo.query/2
        // - Repo.all/1 → Repo.query/2
        // - Logger.log_query/2 → Repo.insert/1
        let cmd = CallsFromCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // With CallDirection::From, MyApp.Repo has 3 outgoing calls.
        // With CallDirection::To (wrong), it would have 5 incoming calls.
        assert_eq!(
            result.total_items, 3,
            "MyApp.Repo should have exactly 3 outgoing calls (not 5 incoming)"
        );

        // Verify the grouped module is the caller module (MyApp.Repo)
        assert_eq!(result.items.len(), 1, "Should have exactly 1 module group");
        assert_eq!(
            result.items[0].name, "MyApp.Repo",
            "Module group should be the caller module"
        );

        // Verify the entries are caller functions from Repo
        let func_names: HashSet<_> = result.items[0]
            .entries
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(
            func_names.contains("get"),
            "Should contain caller function 'get'"
        );
        assert!(
            func_names.contains("all"),
            "Should contain caller function 'all'"
        );
        assert!(
            func_names.contains("insert"),
            "Should contain caller function 'insert'"
        );

        // Verify each entry has callee info (outgoing target), not caller info
        let mut callee_targets: HashSet<(String, String, i64)> = HashSet::new();
        for func in &result.items[0].entries {
            for call in &func.calls {
                callee_targets.insert((
                    call.callee.module.to_string(),
                    call.callee.name.to_string(),
                    call.callee.arity,
                ));
            }
        }
        assert!(
            callee_targets.contains(&("MyApp.Repo".to_string(), "query".to_string(), 2)),
            "Should contain outgoing target Repo.query/2"
        );
        assert!(
            callee_targets.contains(&("MyApp.Service".to_string(), "get_context".to_string(), 1)),
            "Should contain outgoing target Service.get_context/1"
        );
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_calls_from_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: true,
                limit: 1,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 1, "Limit should restrict to 1 call");
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(
            output.contains("Calls from: MyApp.Accounts"),
            "Table output should contain header, got:\n{}",
            output
        );
        assert!(
            output.contains("MyApp.Accounts"),
            "Table should contain caller module name"
        );
        assert!(
            output.contains("Found 4 call(s):"),
            "Table should contain summary with 4 calls, got:\n{}",
            output
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsFromCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(
            output.contains("Calls from: NonExistent"),
            "Header should contain queried module"
        );
        assert!(
            output.contains("No calls found."),
            "Empty result should show empty message"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Accounts");
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["total_items"], 4);
    }
}
