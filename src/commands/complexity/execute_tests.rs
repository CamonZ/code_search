//! Execute tests for complexity command.

#[cfg(test)]
mod tests {
    use super::super::ComplexityCmd;
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

    // Test with default thresholds (min >= 1, depth >= 0)
    crate::execute_test! {
        test_name: test_complexity_default_thresholds,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // With default thresholds, all functions should be included (default complexity is 1)
            assert_eq!(result.total_items, 15);
            assert_eq!(result.items.len(), 5); // 5 modules
        },
    }

    // Test with higher complexity threshold filters out lower complexity functions
    crate::execute_test! {
        test_name: test_complexity_high_threshold,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 10,
            min_depth: 0,
            exclude_generated: false,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // No functions should exceed complexity 10 with default fixture
            assert_eq!(result.total_items, 0);
            assert!(result.items.is_empty());
        },
    }

    // Test with min depth filter
    crate::execute_test! {
        test_name: test_complexity_min_depth,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 5,
            exclude_generated: false,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // No functions should have depth >= 5 with default fixture
            assert_eq!(result.total_items, 0);
        },
    }

    // Test with module filter
    crate::execute_test! {
        test_name: test_complexity_with_module_filter,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Should only return MyApp.Accounts module (4 functions)
            assert_eq!(result.total_items, 4);
            assert_eq!(result.items.len(), 1);
            assert_eq!(result.items[0].name, "MyApp.Accounts");
            assert_eq!(result.items[0].entries.len(), 4);
        },
    }

    // Test with module regex filter
    crate::execute_test! {
        test_name: test_complexity_with_module_regex,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: Some("MyApp\\..*".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            // Should return all MyApp.* modules
            assert_eq!(result.total_items, 15);
            assert_eq!(result.items.len(), 5);
        },
    }

    // Test limit parameter
    crate::execute_test! {
        test_name: test_complexity_with_limit,
        fixture: populated_db,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 5,
            },
        },
        assertions: |result| {
            // With limit of 5, should get at most 5 functions
            assert_eq!(result.total_items, 5);
        },
    }

    // =========================================================================
    // Empty database tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: ComplexityCmd,
        cmd: ComplexityCmd {
            min: 1,
            min_depth: 0,
            exclude_generated: false,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}
