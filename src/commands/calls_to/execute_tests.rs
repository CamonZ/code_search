//! Execute tests for calls-to command.

#[cfg(test)]
mod tests {
    use super::super::CallsToCmd;
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert!(result.items.is_empty(), "Expected no results for non-existent arity");
            assert_eq!(result.total_items, 0);
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_calls_to_with_project_filter,
        fixture: populated_db,
        cmd: CallsToCmd {
            module: "MyApp.Repo".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
