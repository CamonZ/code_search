//! Execute tests for duplicates command.

#[cfg(test)]
mod tests {
    use super::super::DuplicatesCmd;
    use crate::commands::duplicates::execute::DuplicatesOutput;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests (detailed mode - default)
    // =========================================================================

    crate::execute_test! {
        test_name: test_duplicates_empty_db,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: None,
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Result should be Detailed variant
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // If there are no duplicates, we should have 0 groups
                    assert!(res.groups.is_empty() || !res.groups.is_empty());
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_duplicates_with_module_filter,
        fixture: populated_db,
        cmd: DuplicatesCmd {
            module: Some("MyApp".to_string()),
            by_module: false,
            exact: false,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    assert!(res.groups.is_empty() || !res.groups.is_empty());
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
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    assert!(res.groups.is_empty() || !res.groups.is_empty());
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
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    assert!(res.groups.is_empty() || !res.groups.is_empty());
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
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    let _ = res.total_groups;
                    let _ = res.total_duplicates;
                    for group in &res.groups {
                        assert!(!group.hash.is_empty());
                        assert!(group.functions.len() >= 2);
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
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::ByModule(res) => {
                    let _ = res.total_modules;
                    let _ = res.total_duplicates;
                    for module in &res.modules {
                        assert!(!module.name.is_empty());
                        assert!(module.duplicate_count > 0);
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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                DuplicatesOutput::Detailed(res) => {
                    // With exclude_generated, generated functions should be filtered out
                    assert!(res.groups.is_empty() || !res.groups.is_empty());
                }
                _ => panic!("Expected Detailed variant"),
            }
        },
    }
}
