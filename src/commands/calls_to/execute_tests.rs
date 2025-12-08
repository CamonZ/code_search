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
    crate::execute_count_test! {
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
        field: calls,
        expected: 4,
    }

    // 3 calls to Repo.get: from get_user/1, get_user/2, do_fetch
    crate::execute_count_test! {
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
        field: calls,
        expected: 3,
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
            assert_eq!(result.calls.len(), 3);
            assert!(result.calls.iter().all(|c| c.callee_arity == 2));
        },
    }

    // 4 calls match get|all: 3 to get + 1 to all
    crate::execute_count_test! {
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
        field: calls,
        expected: 4,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
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
        empty_field: calls,
    }

    crate::execute_no_match_test! {
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
        empty_field: calls,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
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
        collection: calls,
        condition: |c| c.project == "test_project",
    }

    crate::execute_limit_test! {
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
        collection: calls,
        limit: 2,
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
