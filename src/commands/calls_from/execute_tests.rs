//! Execute tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::CallsFromCmd;
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

    // MyApp.Accounts has 3 call records: get_user/1→Repo.get, get_user/2→Repo.get, list_users→Repo.all
    // Per-function deduplication: each function keeps its unique callees = 3 calls displayed
    crate::execute_test! {
        test_name: test_calls_from_module,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3,
                "Expected 3 displayed calls from MyApp.Accounts (1 per caller function)");
        },
    }

    // get_user functions (both arities) call Repo.get
    // Per-function deduplication: get_user/1 has 1 call, get_user/2 has 1 call = 2 displayed
    crate::execute_test! {
        test_name: test_calls_from_function,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2,
                "Expected 2 displayed calls (1 from each get_user arity)");
            // Check that all calls target MyApp.Repo.get
            for module in &result.items {
                for func in &module.entries {
                    for call in &func.calls {
                        assert_eq!(call.callee.module.as_ref(), "MyApp.Repo");
                        assert_eq!(call.callee.name.as_ref(), "get");
                    }
                }
            }
        },
    }

    // All 11 calls in the fixture are from MyApp.* modules
    // Per-function deduplication: each caller keeps unique callees = 11 displayed
    crate::execute_test! {
        test_name: test_calls_from_regex_module,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 11,
                "Expected 11 displayed calls from MyApp.* modules");
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_calls_from_no_match,
        fixture: populated_db,
        cmd: CallsFromCmd {
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
            assert!(result.items.is_empty(), "Expected no modules for non-existent module");
            assert_eq!(result.total_items, 0);
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_calls_from_with_project_filter,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // All results should be for the test_project (verified implicitly by getting results)
            assert!(result.total_items > 0, "Should have calls with project filter");
        },
    }

    crate::execute_test! {
        test_name: test_calls_from_with_limit,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 1,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 1, "Limit should restrict to 1 call");
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: CallsFromCmd,
        cmd: CallsFromCmd {
            module: "MyApp".to_string(),
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
