//! Execute tests for duplicates command.

#[cfg(test)]
mod tests {
    use super::super::DuplicatesCmd;
    use crate::commands::duplicates::execute::DuplicatesOutput;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests (detailed mode - default)
    // =========================================================================

    crate::execute_test! {
        test_name: test_duplicates_default_finds_all_groups,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // The surreal fixture has 2 AST hash groups: ast_hash_001 (2 funcs) and
            // ast_hash_002 (2 generated funcs) = 2 groups, 4 total duplicates
            match result {
                DuplicatesOutput::Detailed(res) => {
                    assert_eq!(res.total_groups, 2, "Should have 2 duplicate groups");
                    assert_eq!(res.total_duplicates, 4, "Should have 4 total duplicates");
                    assert_eq!(res.groups.len(), 2, "Groups vec should have 2 entries");

                    // Verify each group has the expected hash and member count
                    let group_001 = res.groups.iter().find(|g| g.hash == "ast_hash_001")
                        .expect("Should have group with ast_hash_001");
                    assert_eq!(group_001.functions.len(), 2,
                        "ast_hash_001 group should have 2 functions");

                    let group_002 = res.groups.iter().find(|g| g.hash == "ast_hash_002")
                        .expect("Should have group with ast_hash_002");
                    assert_eq!(group_002.functions.len(), 2,
                        "ast_hash_002 group should have 2 functions");
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_module_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("Accounts".to_string()),
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // Accounts has format_name/1 (ast_hash_001) and __generated__/0 (ast_hash_002)
                    assert!(!res.groups.is_empty(), "Should find duplicates for Accounts");
                    for group in &res.groups {
                        for func in &group.functions {
                            assert!(func.module.contains("Accounts"),
                                "All functions should be in Accounts module, got: {}", func.module);
                        }
                    }
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_exact_flag,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: false,
            exact: true,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // Exact mode uses source_sha; fixture has src_hash_001 pair
                    assert_eq!(res.total_groups, 1, "Exact mode should find 1 source hash group");
                    assert_eq!(res.total_duplicates, 2, "Exact mode should find 2 duplicates");
                    assert_eq!(res.groups[0].hash, "src_hash_001");
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_regex_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // Controller has format_display/1 (ast_hash_001) and __generated__/0 (ast_hash_002)
                    assert!(!res.groups.is_empty(), "Should find duplicates for Controller");
                    for group in &res.groups {
                        for func in &group.functions {
                            assert_eq!(func.module, "MyApp.Controller",
                                "Regex should match only MyApp.Controller exactly");
                        }
                    }
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_structure,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    assert_eq!(res.total_groups, res.groups.len(),
                        "total_groups should match groups.len()");
                    let sum: usize = res.groups.iter().map(|g| g.functions.len()).sum();
                    assert_eq!(res.total_duplicates, sum,
                        "total_duplicates should equal sum of all function counts");
                    for group in &res.groups {
                        assert!(!group.hash.is_empty());
                        assert!(group.functions.len() >= 2,
                            "Each duplicate group must have at least 2 functions");
                    }
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    // =========================================================================
    // By-module mode tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_duplicates_by_module,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: true,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::ByModule(res) => {
                    assert!(res.total_modules > 0, "Should find modules with duplicates");
                    assert!(res.total_duplicates > 0, "Should find total duplicates");
                    for module in &res.modules {
                        assert!(!module.name.is_empty());
                        assert!(module.duplicate_count > 0,
                            "Each module should have at least 1 duplicate");
                        // Verify copy counts are correctly accumulated via +=
                        for dup in &module.top_duplicates {
                            assert!(dup.copy_count >= 1,
                                "copy_count should be >= 1, got {}", dup.copy_count);
                        }
                    }
                }
                _ => panic!("Expected ByModule variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_by_module_with_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("MyApp".to_string()),
            by_module: true,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::ByModule(res) => {
                    for module in &res.modules {
                        assert!(module.name.contains("MyApp"));
                    }
                }
                _ => panic!("Expected ByModule variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_exclude_generated,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: true,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // With exclude_generated, only ast_hash_001 group remains (2 funcs)
                    assert_eq!(res.total_groups, 1,
                        "Excluding generated should leave 1 group");
                    assert_eq!(res.total_duplicates, 2,
                        "Excluding generated should leave 2 duplicates");
                    assert_eq!(res.groups[0].hash, "ast_hash_001",
                        "Remaining group should be ast_hash_001");
                    for func in &res.groups[0].functions {
                        assert!(!func.name.contains("__generated__"),
                            "Should not contain generated functions");
                    }
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    // =========================================================================
    // build_by_module_result: verify += accumulation (catches -= and *= mutants)
    // =========================================================================

    crate::execute_test! {
        test_name: test_by_module_copy_count_accumulation,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: true,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // This test verifies that += correctly accumulates copy counts.
            // With -= mutant: counts would be negative. With *= mutant: counts would be 0.
            match result {
                DuplicatesOutput::ByModule(res) => {
                    // The fixture has AST duplicates across Accounts and Controller.
                    // Each module should have positive copy counts that sum correctly.
                    let total_copies: i64 = res.modules.iter()
                        .flat_map(|m| m.top_duplicates.iter())
                        .map(|d| d.copy_count)
                        .sum();
                    assert!(total_copies > 0,
                        "Total copy counts must be positive (catches -= and *= mutants), got {}",
                        total_copies);

                    // Each individual copy_count must be exactly 1 (each function
                    // name/arity appears once per module in this fixture)
                    for module in &res.modules {
                        for dup in &module.top_duplicates {
                            assert_eq!(dup.copy_count, 1,
                                "Function {}/{} in {} should have copy_count=1, got {}",
                                dup.name, dup.arity, module.name, dup.copy_count);
                        }
                    }
                }
                _ => panic!("Expected ByModule variant"),
            }
        },
    }

    // =========================================================================
    // run() integration test: verify formatted output (catches run() mutants)
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };

        let output = cmd.run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // The output must contain meaningful content, not empty or "xyzzy"
        assert!(output.contains("Duplicate Functions"),
            "run() output should contain header 'Duplicate Functions'");
        assert!(output.contains("Group"),
            "run() output should contain group listings");
        assert!(!output.is_empty(),
            "run() output must not be empty");
    }

    #[rstest]
    fn test_run_json_produces_valid_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };

        let output = cmd.run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        // Verify it's valid JSON with expected fields
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("run() JSON output should be valid JSON");
        assert!(parsed.get("total_groups").is_some(),
            "JSON output should have total_groups field");
        assert!(parsed.get("groups").is_some(),
            "JSON output should have groups field");
    }
}
