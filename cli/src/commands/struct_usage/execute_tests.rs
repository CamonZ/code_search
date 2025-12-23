//! Execute tests for struct-usage command.

#[cfg(test)]
mod tests {
    use super::super::StructUsageCmd;
    use super::super::execute::StructUsageOutput;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: type_signatures,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests - Detailed mode
    // =========================================================================

    // The type_signatures fixture has User.t() in returns for:
    // - MyApp.Accounts: get_user/1, get_user/2, list_users/0, create_user/1
    // - MyApp.Users: get_by_email/1, authenticate/2
    crate::execute_test! {
        test_name: test_struct_usage_finds_user_type,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: ".*User\\.t.*".to_string(), // Use regex for substring matching
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert!(detail.total_items > 0, "Should find functions using User.t");
                    // Should have entries from at least 2 modules
                    assert!(detail.items.len() >= 2, "Should find User.t in multiple modules");
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_struct_usage_with_module_filter,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: ".*User\\.t.*".to_string(), // Use regex for substring matching
            module: Some("MyApp.Accounts".to_string()),
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert!(detail.total_items > 0, "Should find functions in MyApp.Accounts");
                    // All results should be from MyApp.Accounts
                    for module_group in &detail.items {
                        assert_eq!(module_group.name, "MyApp.Accounts");
                    }
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    // =========================================================================
    // Core functionality tests - ByModule mode
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_usage_by_module,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: ".*User\\.t.*".to_string(), // Use regex for substring matching
            module: None,
            by_module: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::ByModule(ref by_module) => {
                    assert!(by_module.total_modules > 0, "Should find modules using User.t");
                    assert!(by_module.total_functions > 0, "Should have function count");
                    // Each module should have counts
                    for module in &by_module.modules {
                        assert!(module.total > 0, "Module should have at least one function");
                    }
                }
                _ => panic!("Expected ByModule output"),
            }
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_usage_no_match,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: "NonExistentType.t".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert!(detail.items.is_empty(), "Should find no matches");
                    assert_eq!(detail.total_items, 0);
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_struct_usage_by_module_no_match,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: "NonExistentType.t".to_string(),
            module: None,
            by_module: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::ByModule(ref by_module) => {
                    assert!(by_module.modules.is_empty(), "Should find no modules");
                    assert_eq!(by_module.total_modules, 0);
                    assert_eq!(by_module.total_functions, 0);
                }
                _ => panic!("Expected ByModule output"),
            }
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_usage_with_limit,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: ".*User\\.t.*".to_string(), // Use regex for substring matching
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 1,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert_eq!(detail.total_items, 1, "Limit should restrict to 1 result");
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    crate::execute_test! {
        test_name: test_struct_usage_regex_pattern,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: ".*\\.t\\(\\)".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    // Should match User.t(), Ecto.Changeset.t(), etc.
                    assert!(detail.total_items > 0, "Regex should match .t() types");
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    // Exact type match - search for integer() in inputs
    crate::execute_test! {
        test_name: test_struct_usage_exact_match,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: "integer()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert!(detail.total_items > 0, "Should find exact match for integer()");
                    // Verify we found functions using integer()
                    assert!(detail.items.len() >= 1, "Should find integer() in at least one module");
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    // Exact match doesn't find partial matches
    crate::execute_test! {
        test_name: test_struct_usage_exact_no_partial,
        fixture: populated_db,
        cmd: StructUsageCmd {
            pattern: "integer".to_string(), // Won't match "integer()" - missing parens
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            match result {
                StructUsageOutput::Detailed(ref detail) => {
                    assert_eq!(detail.total_items, 0, "Exact match should not find partial matches");
                    assert!(detail.items.is_empty());
                }
                _ => panic!("Expected Detailed output"),
            }
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: StructUsageCmd,
        cmd: StructUsageCmd {
            pattern: "User.t".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}
