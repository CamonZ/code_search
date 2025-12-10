//! Execute tests for trace command.

#[cfg(test)]
mod tests {
    use super::super::TraceCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Controller.index only calls Accounts.list_users at depth 1
    crate::execute_test! {
        test_name: test_trace_single_depth,
        fixture: populated_db,
        cmd: TraceCmd {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            depth: 1,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_calls, 1);
            assert_eq!(result.roots.len(), 1);
            assert_eq!(result.roots[0].calls[0].module, "MyApp.Accounts");
            assert_eq!(result.roots[0].calls[0].function, "list_users");
        },
    }

    // Controller.index -> list_users -> all (2 steps with depth 2)
    crate::execute_test! {
        test_name: test_trace_multiple_depths,
        fixture: populated_db,
        cmd: TraceCmd {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            depth: 3,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_calls, 2);
        },
    }

    crate::execute_test! {
        test_name: test_trace_with_depth_limit,
        fixture: populated_db,
        cmd: TraceCmd {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            depth: 2,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_calls, 2);
            assert!(result.max_depth <= 2);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_trace_no_match,
        fixture: populated_db,
        cmd: TraceCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            depth: 5,
            limit: 100,
        },
        empty_field: roots,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: TraceCmd,
        cmd: TraceCmd {
            module: "MyApp".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            depth: 5,
            limit: 100,
        },
    }
}
