//! Execute tests for god_modules command.

#[cfg(test)]
mod tests {
    use super::super::GodModulesCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // The complex fixture has 9 modules with various connectivity:
    // - Controller: 5 outgoing, 1 incoming = 6 total
    // - Accounts: 4 outgoing, 4 incoming = 8 total
    // - Service: 3 outgoing, 2 incoming = 5 total
    // - Repo: 3 outgoing, 3 incoming = 6 total
    // - Notifier: 1 outgoing, 3 incoming = 4 total
    // - etc.

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
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind_filter, Some("god".to_string()));
        // Should have modules that meet the criteria
        assert!(result.total_items > 0, "Should find modules with connectivity");
    }

    #[rstest]
    fn test_god_modules_finds_connected_modules(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 4, // At least 4 total calls
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Multiple modules have >= 4 total connectivity
        assert!(
            result.total_items >= 3,
            "Should find at least 3 modules with >= 4 total calls"
        );

        // Verify all results meet threshold
        for item in &result.items {
            let entry = &item.entries[0];
            assert!(
                entry.total >= 4,
                "Module {} has {} total, expected >= 4",
                item.name,
                entry.total
            );
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
                    result.items[i].name,
                    current_total,
                    result.items[i + 1].name,
                    next_total
                );
            }
        }
    }

    #[rstest]
    fn test_god_modules_entry_structure(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for item in &result.items {
            // Each module should have exactly one entry
            assert_eq!(
                item.entries.len(),
                1,
                "Module {} should have exactly one entry",
                item.name
            );

            let entry = &item.entries[0];
            // All counts should be non-negative
            assert!(entry.function_count >= 0);
            assert!(entry.loc >= 0);
            assert!(entry.incoming >= 0);
            assert!(entry.outgoing >= 0);
            assert!(entry.total >= 0);

            // Total should equal incoming + outgoing
            assert_eq!(entry.total, entry.incoming + entry.outgoing);
        }
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_god_modules_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: Some("Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All results should contain "Accounts"
        for item in &result.items {
            assert!(
                item.name.contains("Accounts"),
                "Module {} doesn't contain 'Accounts'",
                item.name
            );
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
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(
            result.items.len() <= 2,
            "Expected at most 2 results, got {}",
            result.items.len()
        );
    }

    #[rstest]
    fn test_god_modules_high_threshold_filters_out(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 999999,
            min_loc: 999999,
            min_total: 999999,
            module: None,
            common: CommonArgs {
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
    fn test_god_modules_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 1,
            module: Some("NonExistentModule".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
    }

    #[rstest]
    fn test_god_modules_combined_thresholds(populated_db: Box<dyn db::backend::Database>) {
        let cmd = GodModulesCmd {
            min_functions: 2,
            min_loc: 2,
            min_total: 2,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All results must satisfy ALL three criteria
        for item in &result.items {
            let entry = &item.entries[0];
            assert!(
                entry.function_count >= 2,
                "Module {} has {} functions, expected >= 2",
                item.name,
                entry.function_count
            );
            assert!(
                entry.loc >= 2,
                "Module {} has {} LoC, expected >= 2",
                item.name,
                entry.loc
            );
            assert!(
                entry.total >= 2,
                "Module {} has {} total, expected >= 2",
                item.name,
                entry.total
            );
        }
    }
}
