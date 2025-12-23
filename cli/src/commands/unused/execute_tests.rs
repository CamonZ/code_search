//! Execute tests for unused command.

#[cfg(test)]
mod tests {
    use super::super::UnusedCmd;
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

    // Uncalled functions: index, show, create (Controller), get_user/2 + validate_email (Accounts), insert (Repo) = 6
    // Note: get_user/1 is called but get_user/2 is not (Controller.show calls arity 1 only)
    crate::execute_test! {
        test_name: test_unused_finds_uncalled_functions,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 6);
            let all_funcs: Vec<&str> = result.items.iter()
                .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
                .collect();
            assert!(all_funcs.contains(&"validate_email"));
            assert!(all_funcs.contains(&"insert"));
        },
    }

    // In Accounts: validate_email (defp) and get_user/2 (def, not called) = 2
    crate::execute_test! {
        test_name: test_unused_with_module_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some(".*Accounts.*".to_string()), // Use regex for substring matching
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2);
        },
    }

    // Controller has 3 uncalled functions
    crate::execute_test! {
        test_name: test_unused_with_regex_filter,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 3);
        },
    }

    // Exact module match - MyApp.Accounts has 2 uncalled functions
    crate::execute_test! {
        test_name: test_unused_exact_module_match,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("MyApp.Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 2);
            // Verify all results are from MyApp.Accounts
            for module_group in &result.items {
                assert_eq!(module_group.name, "MyApp.Accounts");
            }
        },
    }

    // Exact match doesn't find partial matches
    crate::execute_no_match_test! {
        test_name: test_unused_exact_no_partial,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("Accounts".to_string()), // Won't match "MyApp.Accounts"
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_unused_no_match,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: Some("NonExistent".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_unused_with_limit,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 1,
            },
        },
        assertions: |result| {
            // Limit applies to raw results before grouping
            assert_eq!(result.total_items, 1);
        },
    }

    // validate_email is the only private (defp) uncalled function
    crate::execute_test! {
        test_name: test_unused_private_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 1);
            assert_eq!(result.items[0].entries[0].name, "validate_email");
            assert_eq!(result.items[0].entries[0].kind, "defp");
        },
    }

    // 5 public uncalled: index, show, create (Controller), get_user/2 (Accounts), insert (Repo)
    crate::execute_test! {
        test_name: test_unused_public_only,
        fixture: populated_db,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.total_items, 5);
            for module in &result.items {
                for func in &module.entries {
                    assert_eq!(func.kind, "def");
                }
            }
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: UnusedCmd,
        cmd: UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}
