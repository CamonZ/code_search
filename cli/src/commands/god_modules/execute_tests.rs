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

    // =========================================================================
    // Boundary threshold tests (kills < vs == vs <= mutants)
    // =========================================================================

    /// Tests that min_functions uses strict less-than (<).
    /// When min_functions equals a module's func_count, the module should be included.
    /// Kills: execute.rs:51 < replaced with == (would exclude matching values)
    /// Kills: execute.rs:51 < replaced with <= (would include values one below threshold)
    #[rstest]
    fn test_min_functions_boundary(populated_db: Box<dyn db::backend::Database>) {
        // First, discover actual function counts with no filter
        let discover_cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: 0,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let all_results = discover_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(!all_results.items.is_empty(), "Should have modules in fixture");

        // Find a module and its exact function count
        let target = &all_results.items[0];
        let exact_func_count = target.entries[0].function_count;

        // Test at exact boundary: min_functions == func_count should INCLUDE it
        // (because the code checks `func_count < min_functions`, so equal passes)
        let at_boundary_cmd = GodModulesCmd {
            min_functions: exact_func_count,
            min_loc: 0,
            min_total: 0,
            module: Some(target.name.clone()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let at_boundary = at_boundary_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            at_boundary.items.iter().any(|m| m.name == target.name),
            "Module {} with func_count={} should be included when min_functions={}",
            target.name,
            exact_func_count,
            exact_func_count
        );

        // Test one above boundary: min_functions == func_count + 1 should EXCLUDE it
        let above_boundary_cmd = GodModulesCmd {
            min_functions: exact_func_count + 1,
            min_loc: 0,
            min_total: 0,
            module: Some(target.name.clone()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let above_boundary = above_boundary_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            !above_boundary.items.iter().any(|m| m.name == target.name),
            "Module {} with func_count={} should be excluded when min_functions={}",
            target.name,
            exact_func_count,
            exact_func_count + 1
        );
    }

    /// Tests that min_loc uses strict less-than (<).
    /// Kills: execute.rs:59 < replaced with == (would exclude matching values)
    #[rstest]
    fn test_min_loc_boundary(populated_db: Box<dyn db::backend::Database>) {
        // Discover a module with known LoC > 0
        let discover_cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 1,
            min_total: 0,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let results = discover_cmd.execute(&*populated_db).expect("Execute should succeed");

        // Find a module with loc > 0
        if let Some(target) = results.items.iter().find(|m| m.entries[0].loc > 0) {
            let exact_loc = target.entries[0].loc;

            // At exact boundary: should be included
            let at_cmd = GodModulesCmd {
                min_functions: 1,
                min_loc: exact_loc,
                min_total: 0,
                module: Some(target.name.clone()),
                common: CommonArgs {
                    regex: false,
                    limit: 100,
                },
            };
            let at_result = at_cmd.execute(&*populated_db).expect("Execute should succeed");
            assert!(
                at_result.items.iter().any(|m| m.name == target.name),
                "Module {} with loc={} should be included when min_loc={}",
                target.name,
                exact_loc,
                exact_loc
            );

            // One above: should be excluded
            let above_cmd = GodModulesCmd {
                min_functions: 1,
                min_loc: exact_loc + 1,
                min_total: 0,
                module: Some(target.name.clone()),
                common: CommonArgs {
                    regex: false,
                    limit: 100,
                },
            };
            let above_result = above_cmd.execute(&*populated_db).expect("Execute should succeed");
            assert!(
                !above_result.items.iter().any(|m| m.name == target.name),
                "Module {} with loc={} should be excluded when min_loc={}",
                target.name,
                exact_loc,
                exact_loc + 1
            );
        }
    }

    /// Tests that min_total uses strict less-than (<).
    /// Kills: execute.rs:71 < replaced with <= (would include values one below threshold)
    #[rstest]
    fn test_min_total_boundary(populated_db: Box<dyn db::backend::Database>) {
        // Discover all modules with their total connectivity
        let discover_cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: 1,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let results = discover_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(!results.items.is_empty(), "Should have connected modules");

        let target = &results.items[0];
        let exact_total = target.entries[0].total;

        // At exact boundary: should be included
        let at_cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: exact_total,
            module: Some(target.name.clone()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let at_result = at_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            at_result.items.iter().any(|m| m.name == target.name),
            "Module {} with total={} should be included when min_total={}",
            target.name,
            exact_total,
            exact_total
        );

        // One above boundary: should be excluded
        let above_cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: exact_total + 1,
            module: Some(target.name.clone()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let above_result = above_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            !above_result.items.iter().any(|m| m.name == target.name),
            "Module {} with total={} should be excluded when min_total={}",
            target.name,
            exact_total,
            exact_total + 1
        );
    }

    // =========================================================================
    // Scoring formula test (kills + vs * mutant)
    // =========================================================================

    /// Tests that total = incoming + outgoing, not incoming * outgoing.
    /// Kills: execute.rs:68 + replaced with * (would produce different results)
    ///
    /// If incoming=3 and outgoing=2:
    ///   With +: total = 5
    ///   With *: total = 6
    /// This test verifies the correct additive formula is used.
    #[rstest]
    fn test_scoring_formula_addition_not_multiplication(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: 0,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // For every module, verify total == incoming + outgoing (not incoming * outgoing)
        for item in &result.items {
            let entry = &item.entries[0];
            assert_eq!(
                entry.total,
                entry.incoming + entry.outgoing,
                "Module {} should have total={} ({}+{}) but got {}. \
                 If total equals {} ({}*{}), then + was replaced with *",
                item.name,
                entry.incoming + entry.outgoing,
                entry.incoming,
                entry.outgoing,
                entry.total,
                entry.incoming * entry.outgoing,
                entry.incoming,
                entry.outgoing,
            );
            // Also explicitly check it's NOT multiplication (when values differ)
            if entry.incoming != entry.outgoing
                && entry.incoming != 0
                && entry.outgoing != 0
            {
                assert_ne!(
                    entry.total,
                    entry.incoming * entry.outgoing,
                    "Module {} total should NOT equal incoming*outgoing",
                    item.name
                );
            }
        }
    }

    // =========================================================================
    // run() integration tests (kills mod.rs:49 mutants)
    // =========================================================================

    /// Tests that run() produces non-empty, correct output.
    /// Kills: mod.rs:49 run() -> Ok(String::new()) and Ok("xyzzy")
    #[rstest]
    fn test_run_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: 1,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Must not be empty (kills -> Ok(String::new()))
        assert!(!output.is_empty(), "run() should produce non-empty output");
        // Must contain expected header (kills -> Ok("xyzzy"))
        assert!(
            output.contains("God Modules"),
            "Output should contain 'God Modules', got: {}",
            output
        );
        // Verify it contains actual data
        assert!(
            output.contains("god module(s)"),
            "Output should contain results summary"
        );
    }

    /// Tests run() with empty results to ensure even empty output is correct.
    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

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
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        assert!(
            output.contains("God Modules"),
            "Even empty output should have header"
        );
        assert!(
            output.contains("No god modules found."),
            "Empty output should show empty message"
        );
    }

    /// Tests run() with JSON format to kill mutants via format path.
    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = GodModulesCmd {
            min_functions: 1,
            min_loc: 0,
            min_total: 1,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        // Must be valid JSON (kills String::new() and "xyzzy")
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");
        assert_eq!(parsed["kind_filter"], "god");
        assert!(parsed["total_items"].as_u64().unwrap() > 0);
    }
}
