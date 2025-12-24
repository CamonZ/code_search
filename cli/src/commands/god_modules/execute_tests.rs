//! Execute tests for god_modules command.

#[cfg(test)]
mod tests {
    use super::super::GodModulesCmd;
    use crate::commands::CommonArgs;
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

    #[rstest]
    fn test_god_modules_basic(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind_filter, Some("god".to_string()));
        // Should have some modules that meet the criteria
        assert!(result.total_items > 0);
    }

    #[rstest]
    fn test_god_modules_respects_function_count_threshold(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 100, // Very high threshold
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // With high threshold, might have no results
        for item in &result.items {
            let entry = &item.entries[0];
            assert!(entry.function_count >= 100, "Module {} has {} functions, expected >= 100", item.name, entry.function_count);
        }
    }

    #[rstest]
    fn test_god_modules_respects_loc_threshold(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1000, // High LoC threshold
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for item in &result.items {
            let entry = &item.entries[0];
            assert!(entry.loc >= 1000, "Module {} has {} LoC, expected >= 1000", item.name, entry.loc);
        }
    }

    #[rstest]
    fn test_god_modules_respects_total_threshold(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 10, // Require at least 10 total calls
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for item in &result.items {
            let entry = &item.entries[0];
            assert!(entry.total >= 10, "Module {} has {} total calls, expected >= 10", item.name, entry.total);
            assert_eq!(entry.total, entry.incoming + entry.outgoing, "Total should equal incoming + outgoing");
        }
    }

    #[rstest]
    fn test_god_modules_sorted_by_connectivity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        if result.items.len() > 1 {
            // Check that results are sorted by total connectivity (descending)
            for i in 0..result.items.len() - 1 {
                let current_total = result.items[i].entries[0].total;
                let next_total = result.items[i + 1].entries[0].total;
                assert!(
                    current_total >= next_total,
                    "Results not sorted: {} (total={}) should be >= {} (total={})",
                    result.items[i].name, current_total,
                    result.items[i + 1].name, next_total
                );
            }
        }
    }

    #[rstest]
    fn test_god_modules_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: Some("Accounts".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All results should contain "Accounts"
        for item in &result.items {
            assert!(item.name.contains("Accounts"), "Module {} doesn't contain 'Accounts'", item.name);
        }
    }

    #[rstest]
    fn test_god_modules_respects_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.items.len() <= 2, "Expected at most 2 results, got {}", result.items.len());
    }

    #[rstest]
    fn test_god_modules_entry_structure(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for item in &result.items {
            // Each module should have exactly one entry
            assert_eq!(item.entries.len(), 1, "Module {} should have exactly one entry", item.name);

            let entry = &item.entries[0];
            // All counts should be non-negative
            assert!(entry.function_count >= 0);
            assert!(entry.loc >= 0);
            assert!(entry.incoming >= 0);
            assert!(entry.outgoing >= 0);
            assert!(entry.total >= 0);

            // Total should equal incoming + outgoing
            assert_eq!(entry.total, entry.incoming + entry.outgoing);

            // function_count should be populated
            assert_eq!(item.function_count, Some(entry.function_count));
        }
    }

    #[rstest]
    fn test_god_modules_all_thresholds_filter_everything(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 999999, // Impossible threshold
            min_loc: 999999,
            min_total: 999999,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Should return empty results, not error
        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
    }

    #[rstest]
    fn test_god_modules_module_pattern_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: Some("NonExistentModule".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Should return empty results
        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
        assert_eq!(result.module_pattern, "NonExistentModule");
    }

    #[rstest]
    fn test_god_modules_wrong_project(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "wrong_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Should return empty results for non-existent project
        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
    }

    #[rstest]
    fn test_god_modules_result_metadata(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: Some("Accounts".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Verify result metadata is correct
        assert_eq!(result.module_pattern, "Accounts");
        assert_eq!(result.function_pattern, None);
        assert_eq!(result.kind_filter, Some("god".to_string()));
        assert_eq!(result.name_filter, None);
    }

    #[rstest]
    fn test_god_modules_combined_thresholds(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 2,  // Multiple filters
            min_loc: 10,
            min_total: 2,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All results must satisfy ALL three criteria
        for item in &result.items {
            let entry = &item.entries[0];
            assert!(entry.function_count >= 2, "Module {} has {} functions, expected >= 2", item.name, entry.function_count);
            assert!(entry.loc >= 10, "Module {} has {} LoC, expected >= 10", item.name, entry.loc);
            assert!(entry.total >= 2, "Module {} has {} total, expected >= 2", item.name, entry.total);
        }
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: GodModulesCmd,
        cmd: GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
    }
}
