//! Execute tests for many_clauses command.

#[cfg(test)]
mod tests {
    use super::super::ManyClausesCmd;
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
        test_name: test_many_clauses_low_threshold,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // All 15 functions in the fixture have clauses >= 1
            assert_eq!(result.total_items, 15);
            assert_eq!(result.items.len(), 5); // 5 modules
        },
    }

    // Test that default threshold (5) filters out all fixture functions
    crate::execute_test! {
        test_name: test_many_clauses_default_threshold,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 5,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // No functions in fixture have >= 5 clauses
            assert_eq!(result.total_items, 0);
            assert!(result.items.is_empty());
        },
    }

    // Test that threshold filters: each fixture function has exactly 1 clause,
    // so min_clauses=2 should return nothing (demonstrates threshold filtering works)
    crate::execute_test! {
        test_name: test_many_clauses_threshold_filters_out_single_clause,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 2,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Each function in the call_graph fixture has exactly 1 clause,
            // so min_clauses=2 should filter them all out
            assert_eq!(result.total_items, 0,
                "No fixture functions have >= 2 clauses");
            assert!(result.items.is_empty());
        },
    }

    // Verify all returned entries meet the clause threshold
    crate::execute_test! {
        test_name: test_many_clauses_entries_meet_threshold,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(result.total_items > 0, "Should find functions with >= 1 clause");
            for group in &result.items {
                for entry in &group.entries {
                    assert!(entry.clauses >= 1,
                        "All results should have clauses >= 1, but {}/{} has {} clauses",
                        entry.name, entry.arity, entry.clauses);
                }
            }
        },
    }

    // =========================================================================
    // Module filtering tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_many_clauses_with_module_filter,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        test_name: test_many_clauses_with_module_regex,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        test_name: test_many_clauses_no_match,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
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

    crate::execute_test! {
        test_name: test_module_grouping_no_duplicates,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        cmd: ManyClausesCmd {
            min_clauses: 1,
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

    // This test specifically catches the mutant that removes `!` from
    // `if !module_map.contains_key(&func.module)` at execute.rs:51.
    //
    // A module with exactly one function returned from the DB is the key:
    // - With `!` (correct): the first (only) function triggers push to
    //   module_order because the module is NOT yet in the map.
    // - Without `!` (mutant): the first function does NOT trigger push
    //   because the module is NOT in the map, and there is no second
    //   function to ever trigger the (now-inverted) condition. The module
    //   is completely missing from the final result.
    //
    // We use a custom JSON fixture containing ModuleA (2 functions) and
    // ModuleB (1 function). With the mutant, ModuleB would vanish.
    #[fixture]
    fn single_fn_module_db() -> Box<dyn db::backend::Database> {
        // ModuleA has 2 functions, ModuleB has exactly 1 function.
        // This catches the ! negation mutant because ModuleB would be
        // lost when the negation is removed.
        let json = r#"{
            "generated_at": "2024-01-15T10:30:00.000000Z",
            "project_path": "/test",
            "environment": "dev",
            "extraction_metadata": {
                "modules_processed": 2,
                "modules_with_debug_info": 2,
                "modules_without_debug_info": 0,
                "total_calls": 0,
                "total_functions": 3,
                "total_specs": 0,
                "total_types": 0,
                "total_structs": 0,
                "extraction_time_ms": 1
            },
            "structs": {},
            "function_locations": {
                "ModuleA": {
                    "ModuleA.foo/0:1": {
                        "name": "foo",
                        "arity": 0,
                        "line": 1,
                        "start_line": 1,
                        "end_line": 5,
                        "kind": "def",
                        "guard": null,
                        "pattern": "",
                        "source_file": "lib/module_a.ex",
                        "source_file_absolute": "/test/lib/module_a.ex",
                        "source_sha": "aaa",
                        "ast_sha": "bbb",
                        "generated_by": null,
                        "macro_source": null,
                        "complexity": 1,
                        "max_nesting_depth": 0
                    },
                    "ModuleA.bar/1:10": {
                        "name": "bar",
                        "arity": 1,
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15,
                        "kind": "def",
                        "guard": null,
                        "pattern": "x",
                        "source_file": "lib/module_a.ex",
                        "source_file_absolute": "/test/lib/module_a.ex",
                        "source_sha": "ccc",
                        "ast_sha": "ddd",
                        "generated_by": null,
                        "macro_source": null,
                        "complexity": 1,
                        "max_nesting_depth": 0
                    }
                },
                "ModuleB": {
                    "ModuleB.only/0:1": {
                        "name": "only",
                        "arity": 0,
                        "line": 1,
                        "start_line": 1,
                        "end_line": 3,
                        "kind": "def",
                        "guard": null,
                        "pattern": "",
                        "source_file": "lib/module_b.ex",
                        "source_file_absolute": "/test/lib/module_b.ex",
                        "source_sha": "eee",
                        "ast_sha": "fff",
                        "generated_by": null,
                        "macro_source": null,
                        "complexity": 1,
                        "max_nesting_depth": 0
                    }
                }
            },
            "calls": [],
            "specs": {},
            "types": {}
        }"#;
        db::test_utils::setup_test_db(json)
    }

    // This is the critical test that kills the `delete !` mutant at execute.rs:51.
    // ModuleB has exactly 1 function. With `!` removed, the only function never
    // triggers the push to module_order, so ModuleB is completely absent from results.
    #[rstest]
    fn test_single_function_module_included(single_fn_module_db: Box<dyn db::backend::Database>) {
        use crate::commands::Execute;

        let cmd = ManyClausesCmd {
            min_clauses: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*single_fn_module_db).expect("Execute should succeed");

        // Both modules should be present
        let module_names: Vec<&str> = result.items.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(result.items.len(), 2,
            "Should have both ModuleA and ModuleB, got: {:?}", module_names);
        assert!(
            module_names.contains(&"ModuleB"),
            "ModuleB (single-function module) must be present, got: {:?}",
            module_names
        );
        assert_eq!(result.total_items, 3, "Should have 3 total functions");
    }

    // =========================================================================
    // Limit tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_many_clauses_respects_limit,
        fixture: populated_db,
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
        cmd: ManyClausesCmd {
            min_clauses: 1,
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
                    assert!(entry.clauses >= 1, "clauses should be >= 1");
                    assert!(entry.first_line > 0, "first_line should be positive");
                    assert!(entry.last_line >= entry.first_line, "last_line >= first_line");
                    assert!(!entry.file.is_empty(), "file should not be empty");
                }
            }
        },
    }

    // =========================================================================
    // CommandRunner::run() integration test (kills run() -> Ok("xyzzy") and
    // Ok(String::new()) mutants on mod.rs:44)
    // =========================================================================

    #[rstest]
    fn test_run_returns_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = ManyClausesCmd {
            min_clauses: 1,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd.run(&*populated_db, OutputFormat::Table).expect("run should succeed");
        assert!(output.contains("Functions with Many Clauses"), "run() output should contain the header");
        assert!(output.contains("MyApp."), "run() output should contain module names");
    }

    #[rstest]
    fn test_run_empty_result_contains_empty_message(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = ManyClausesCmd {
            min_clauses: 9999,
            include_generated: false,
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd.run(&*populated_db, OutputFormat::Table).expect("run should succeed");
        assert!(
            output.contains("No functions with many clauses found."),
            "run() with high threshold should show empty message, got: {}",
            output
        );
    }
}
