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
            assert_eq!(result.total_items, 1);
            assert_eq!(result.entries.len(), 2); // Root + 1 callee
            // Entry at index 0 is the root (Controller.index)
            assert_eq!(result.entries[0].module, "MyApp.Controller");
            // Entry at index 1 is the callee (Accounts.list_users)
            assert_eq!(result.entries[1].module, "MyApp.Accounts");
            assert_eq!(result.entries[1].function, "list_users");
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
            assert_eq!(result.total_items, 2);
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
            assert_eq!(result.total_items, 2);
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
        empty_field: entries,
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
