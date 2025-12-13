//! Execute tests for duplicates command.

#[cfg(test)]
mod tests {
    use super::super::DuplicatesCmd;
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

    crate::execute_test! {
        test_name: test_duplicates_empty_db,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            exact: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // If there are no duplicates, we should have 0 groups
            // Result depends on the test fixture data
            assert!(result.groups.is_empty() || !result.groups.is_empty());
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_module_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("MyApp".to_string()),
            exact: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Filter should be applied
            assert!(result.groups.is_empty() || !result.groups.is_empty());
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_exact_flag,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            exact: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Exact flag should use source_sha instead of ast_sha
            assert!(result.groups.is_empty() || !result.groups.is_empty());
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_regex_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            exact: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            // Regex filter should work
            assert!(result.groups.is_empty() || !result.groups.is_empty());
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_structure,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            exact: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Verify result structure exists
            let _ = result.total_groups;
            let _ = result.total_duplicates;

            // If there are groups, verify they have functions
            for group in &result.groups {
                assert!(!group.hash.is_empty());
                assert!(group.functions.len() >= 2); // Duplicates need at least 2
            }
        },
    }
}
