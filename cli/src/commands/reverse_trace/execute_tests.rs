//! Execute tests for reverse-trace command.

#[cfg(test)]
mod tests {
    use super::super::ReverseTraceCmd;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
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
                regex: false,
                limit: 100,
            },
        },
        empty_field: entries,
    }

    // =========================================================================
    // Regex tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_reverse_trace_regex_match,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "MyApp\\.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 1,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3, "Regex match should find same results as exact match");
            assert!(result.entries.iter().all(|e| e.depth == 1));
        },
    }

    crate::execute_no_match_test! {
        test_name: test_reverse_trace_regex_no_match,
        fixture: populated_db,
        cmd: ReverseTraceCmd {
            module: "^NonExistent$".to_string(),
            function: "^xyz$".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        empty_field: entries,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    #[rstest]
    fn test_reverse_trace_invalid_regex(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::Execute;

        let cmd = ReverseTraceCmd {
            module: "[invalid".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };

        let result = cmd.execute(&*populated_db);
        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 1,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(
            output.contains("Reverse trace to: MyApp.Repo.get"),
            "Table output should contain header, got: {}",
            output
        );
        assert!(
            output.contains("Found 3 caller(s) in chain:"),
            "Table output should contain caller count, got: {}",
            output
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = ReverseTraceCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            depth: 5,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        assert!(
            output.contains("Reverse trace to: NonExistent.foo"),
            "Header should contain queried module.function, got: {}",
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

        let cmd = ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            depth: 1,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["module"], "MyApp.Repo");
        assert_eq!(parsed["function"], "get");
        assert_eq!(parsed["direction"], "backward");
        assert!(parsed["entries"].is_array());
        assert_eq!(
            parsed["entries"].as_array().unwrap().len(),
            3,
            "JSON should contain 3 entries"
        );
    }

}
