//! Execute tests for calls-to command.

#[cfg(test)]
mod tests {
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
    // Direction-sensitivity tests
    // =========================================================================

    /// Verify that calls_to returns INCOMING calls (callers), not OUTGOING calls.
    ///
    /// If CallDirection::To were swapped to CallDirection::From, the query for
    /// MyApp.Repo.get would return get's outgoing call (get -> query) instead of
    /// the incoming call (Accounts.get_user -> get). This test catches that swap.
    #[rstest]
    fn test_calls_to_returns_incoming_not_outgoing(populated_db: Box<dyn db::backend::Database>) {
        // MyApp.Repo.get/2 is called BY Accounts.get_user/1 (incoming)
        // MyApp.Repo.get/2 calls Repo.query/2 (outgoing)
        // A direction swap would return the outgoing call instead.
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
            arity: Some(2),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        let func = &result.items[0].entries[0];
        let caller = &func.callers[0];

        // The caller should be Accounts.get_user/1 (incoming direction)
        assert_eq!(
            caller.caller.module.as_ref(), "MyApp.Accounts",
            "Caller should be from MyApp.Accounts (incoming), not MyApp.Repo (outgoing)"
        );
        assert_eq!(
            caller.caller.name.as_ref(), "get_user",
            "Caller should be get_user (incoming), not query (outgoing)"
        );

        // The callee (target function) should be get/2
        assert_eq!(func.name, "get");
        assert_eq!(func.arity, 2);
    }

    /// Verify asymmetry: calls-to and calls-from produce different counts for the same module.
    ///
    /// MyApp.Repo has 5 incoming calls but only 3 outgoing calls. If the direction
    /// were swapped, this test would fail because the count would be 3 instead of 5.
    #[rstest]
    fn test_calls_to_count_differs_from_calls_from(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // MyApp.Repo has 5 incoming calls (calls TO Repo) but only 3 outgoing calls (calls FROM Repo).
        // If the direction were swapped, we'd get 3 instead of 5.
        assert_eq!(
            result.total_items, 5,
            "calls-to MyApp.Repo should return 5 incoming calls, not 3 outgoing"
        );
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
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 2, "Limit should restrict to 2 calls");
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
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
            output.contains("Calls to: MyApp.Repo.get"),
            "Table output should contain header, got: {}",
            output
        );
        assert!(
            output.contains("Found 1 caller(s):"),
            "Table should contain summary, got: {}",
            output
        );
        // Verify the caller direction indicator (incoming arrow)
        assert!(
            output.contains("\u{2190}"),
            "Table should contain incoming arrow indicator, got: {}",
            output
        );
        assert!(
            output.contains("MyApp.Accounts.get_user/1"),
            "Table should show the caller function, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsToCmd {
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
            output.contains("Calls to: NonExistent"),
            "Header should contain queried module, got: {}",
            output
        );
        assert!(
            output.contains("No callers found."),
            "Empty result should show empty message, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: Some("get".to_string()),
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
        assert_eq!(
            parsed["module_pattern"], "MyApp.Repo",
            "JSON should contain module_pattern"
        );
        assert_eq!(
            parsed["total_items"], 1,
            "JSON should contain correct total_items"
        );
    }
}
