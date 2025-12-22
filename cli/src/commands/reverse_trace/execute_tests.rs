//! Execute tests for reverse-trace command.

#[cfg(test)]
mod tests {
    use super::super::ReverseTraceCmd;
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

    // At depth 1: Accounts.get_user/1, Accounts.get_user/2, Service.do_fetch all call Repo.get
    crate::execute_test! {
        test_name: test_reverse_trace_single_depth,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 1,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3);
            // All entries at depth 1 are direct callers of the target
            assert!(result.entries.iter().all(|e| e.depth == 1));
        },
    }

    // Depth 2 adds: Controller.show -> get_user, Service.fetch -> do_fetch
    crate::execute_test! {
        test_name: test_reverse_trace_multiple_depths,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 2,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 5);
        },
    }

    // Trace back from Notifier.send_email (leaf): notify->send_email, process->notify, create->process
    crate::execute_test! {
        test_name: test_reverse_trace_from_leaf,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "MyApp.Notifier".to_string(),
            function: "send_email".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_reverse_trace_no_match,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: entries,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: ReverseTraceCmd,
        cmd: ReverseTraceCmd {
            module: "MyApp".to_string(),
            function: "foo".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}
