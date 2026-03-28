//! Execute tests for large_functions command.

#[cfg(test)]
mod tests {
    use super::super::LargeFunctionsCmd;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Test with low threshold to include all functions
    crate::execute_test! {
        test_name: test_large_functions_low_threshold,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // All 15 functions in the fixture have lines >= 1
            assert_eq!(result.total_items, 15);
            assert_eq!(result.items.len(), 5); // 5 modules
        },
    }

    // Test that default threshold (50) filters out all fixture functions
    crate::execute_test! {
        test_name: test_large_functions_default_threshold,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 50,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // No functions in fixture have >= 50 lines
            assert_eq!(result.total_items, 0);
            assert!(result.items.is_empty());
        },
    }

    // Test with threshold that selects exactly the largest functions
    crate::execute_test! {
        test_name: test_large_functions_threshold_selects_largest,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 10,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Only Controller.create/2 (11 lines) and Service.process/1 (11 lines) have >= 10 lines
            assert_eq!(result.total_items, 2);
            // They come from 2 different modules
            assert_eq!(result.items.len(), 2);
            for group in &result.items {
                for entry in &group.entries {
                    assert!(entry.lines >= 10,
                        "All results should have lines >= 10, but {}/{} has {} lines",
                        entry.name, entry.arity, entry.lines);
                }
            }
        },
    }

    // =========================================================================
    // Module filtering tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_large_functions_with_module_filter,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // MyApp.Accounts has 4 functions
            assert_eq!(result.total_items, 4);
            assert_eq!(result.items.len(), 1);
            assert_eq!(result.items[0].name, "MyApp.Accounts");
            assert_eq!(result.items[0].entries.len(), 4);
        },
    }

    crate::execute_test! {
        test_name: test_large_functions_with_module_regex,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("MyApp\\..*".to_string()),
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            // All modules match MyApp\..* regex
            assert_eq!(result.total_items, 15);
            assert_eq!(result.items.len(), 5);
        },
    }

    // Test no match
    crate::execute_no_match_test! {
        test_name: test_large_functions_no_match,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("NonExistent".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // Module grouping / deduplication tests (kills the ! negation mutant)
    // =========================================================================

    // This test specifically catches the mutant that removes `!` from
    // `if !module_map.contains_key(&func.module)` at execute.rs:52.
    // Without the `!`, module_order would only have entries for modules
    // that were ALREADY seen, skipping the first occurrence of each module.
    // With 4 functions in MyApp.Accounts, removing `!` would push the module
    // name 3 times (for the 2nd, 3rd, 4th function) but miss the 1st push.
    // This test catches it by verifying each module appears exactly once in items.
    crate::execute_test! {
        test_name: test_module_grouping_no_duplicates,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            let module_names: Vec<&str> = result.items.iter().map(|g| g.name.as_str()).collect();
            let unique: std::collections::HashSet<&str> = module_names.iter().copied().collect();
            assert_eq!(
                module_names.len(),
                unique.len(),
                "Each module should appear exactly once in the results, got: {:?}",
                module_names
            );
            assert_eq!(module_names.len(), 5, "Should have exactly 5 unique modules");
        },
    }

    // Verify all entries for a multi-function module are grouped under one ModuleGroup
    crate::execute_test! {
        test_name: test_all_functions_grouped_under_module,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.items.len(), 1, "Should be exactly one module group");
            let group = &result.items[0];
            assert_eq!(group.name, "MyApp.Accounts");
            // All 4 functions should be collected under this single group
            assert_eq!(group.entries.len(), 4,
                "All 4 MyApp.Accounts functions should be in the same group");
        },
    }

    // =========================================================================
    // Limit tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_large_functions_respects_limit,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 5,
            },
        },
        assertions: |result| {
            // Limit of 5 means at most 5 total entries across all modules
            assert_eq!(result.total_items, 5);
        },
    }

    // =========================================================================
    // Module pattern in result
    // =========================================================================

    crate::execute_test! {
        test_name: test_module_pattern_wildcard_when_no_filter,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.module_pattern, "*");
        },
    }

    crate::execute_test! {
        test_name: test_module_pattern_set_when_filter_provided,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("MyApp.Controller".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.module_pattern, "MyApp.Controller");
        },
    }

    // =========================================================================
    // Entry field validation
    // =========================================================================

    crate::execute_test! {
        test_name: test_entry_fields_populated,
        fixture: populated_db,
        cmd: LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: Some("MyApp.Controller".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(!result.items.is_empty());
            for group in &result.items {
                for entry in &group.entries {
                    assert!(!entry.name.is_empty(), "name should not be empty");
                    assert!(entry.arity >= 0, "arity should be non-negative");
                    assert!(entry.start_line > 0, "start_line should be positive");
                    assert!(entry.end_line >= entry.start_line, "end_line >= start_line");
                    assert!(entry.lines > 0, "lines should be positive");
                    assert!(!entry.file.is_empty(), "file should not be empty");
                }
            }
        },
    }

    // =========================================================================
    // CommandRunner::run() integration test (kills run() -> Ok("xyzzy") and
    // Ok(String::new()) mutants on mod.rs:43)
    // =========================================================================

    #[rstest]
    fn test_run_returns_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = LargeFunctionsCmd {
            min_lines: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd.run(&*populated_db, OutputFormat::Table).expect("run should succeed");
        assert!(output.contains("Large Functions"), "run() output should contain the header");
        assert!(output.contains("MyApp."), "run() output should contain module names");
    }

    #[rstest]
    fn test_run_empty_result_contains_empty_message(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = LargeFunctionsCmd {
            min_lines: 9999,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd.run(&*populated_db, OutputFormat::Table).expect("run should succeed");
        assert!(
            output.contains("No large functions found."),
            "run() with high threshold should show empty message, got: {}",
            output
        );
    }
}
