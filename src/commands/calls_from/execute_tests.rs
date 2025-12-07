//! Execute tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::execute::CallsFromResult;
    use super::super::CallsFromCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // MyApp.Accounts has 3 outgoing calls: get_user/1→Repo.get, get_user/2→Repo.get, list_users→Repo.all
    crate::execute_count_test! {
        test_name: test_calls_from_module,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        field: calls,
        expected: 3,
    }

    // get_user functions (both arities) call Repo.get
    crate::execute_test! {
        test_name: test_calls_from_function,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.calls.len(), 2);
            assert!(result.calls.iter().all(|c| c.callee_module == "MyApp.Repo"));
            assert!(result.calls.iter().all(|c| c.callee_function == "get"));
        },
    }

    // All 11 calls in the fixture are from MyApp.* modules
    crate::execute_count_test! {
        test_name: test_calls_from_regex_module,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: calls,
        expected: 11,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_calls_from_no_match,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
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
        test_name: test_calls_from_with_project_filter,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
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
        test_name: test_calls_from_with_limit,
        fixture: populated_db,
        cmd: CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 1,
        },
        collection: calls,
        limit: 1,
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
