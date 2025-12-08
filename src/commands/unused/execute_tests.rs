//! Execute tests for unused command.

#[cfg(test)]
mod tests {
    use super::super::UnusedCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Uncalled functions: index, show, create (Controller), get_user/2 + validate_email (Accounts), insert (Repo) = 6
    // Note: get_user/1 is called but get_user/2 is not (Controller.show calls arity 1 only)
    crate::execute_test! {
        test_name: test_unused_finds_uncalled_functions,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 6);
            let names: Vec<&str> = result.functions.iter().map(|f| f.name.as_str()).collect();
            assert!(names.contains(&"validate_email"));
            assert!(names.contains(&"insert"));
        },
    }

    // In Accounts: validate_email (defp) and get_user/2 (def, not called) = 2
    crate::execute_test! {
        test_name: test_unused_with_module_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("Accounts".to_string()),
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 2);
        },
    }

    // Controller has 3 uncalled functions
    crate::execute_test! {
        test_name: test_unused_with_regex_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            project: "test_project".to_string(),
            regex: true,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 3);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_unused_no_match,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("NonExistent".to_string()),
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
        empty_field: functions,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_limit_test! {
        test_name: test_unused_with_limit,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 1,
        },
        collection: functions,
        limit: 1,
    }

    // validate_email is the only private (defp) uncalled function
    crate::execute_test! {
        test_name: test_unused_private_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 1);
            assert_eq!(result.functions[0].name, "validate_email");
            assert_eq!(result.functions[0].kind, "defp");
        },
    }

    // 5 public uncalled: index, show, create (Controller), get_user/2 (Accounts), insert (Repo)
    crate::execute_test! {
        test_name: test_unused_public_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 5);
            assert!(result.functions.iter().all(|f| f.kind == "def"));
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: UnusedCmd,
        cmd: UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        },
    }
}
