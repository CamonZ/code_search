//! Execute tests for struct-usage command.

/// CozoDB tests use JSON-based fixtures
#[cfg(all(test, not(feature = "backend-surrealdb")))]
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

/// SurrealDB tests use programmatically created fixtures
#[cfg(all(test, feature = "backend-surrealdb"))]
mod tests_surrealdb {
    use super::super::execute::StructUsageOutput;
    use super::super::StructUsageCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    // The SurrealDB specs fixture contains:
    // - 12 total specs (9 @spec + 3 @callback)
    // - user() in 6 specs (return types, all in MyApp.Accounts)
    // - integer() in 3 specs (input types)
    // - String.t() in 2 specs (input types)
    // - Ecto.Queryable.t() in 1 spec (input types)

    #[fixture]
    fn populated_db() -> Box<dyn db::backend::Database> {
        db::test_utils::surreal_specs_db()
    }

    // =========================================================================
    // Core functionality tests - Detailed mode
    // =========================================================================

    // user() appears in 6 specs (all return types in MyApp.Accounts)
    #[rstest]
    fn test_struct_usage_finds_user_type(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 6, "Should find 6 functions using user()");
                // All should be from MyApp.Accounts
                assert_eq!(detail.items.len(), 1, "All user() functions in one module");
                assert_eq!(detail.items[0].name, "MyApp.Accounts");
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    #[rstest]
    fn test_struct_usage_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: Some("MyApp.Accounts".to_string()),
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 6, "Should find 6 functions in MyApp.Accounts");
                for module_group in &detail.items {
                    assert_eq!(module_group.name, "MyApp.Accounts");
                }
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    // integer() appears in 3 specs (input types)
    #[rstest]
    fn test_struct_usage_finds_integer_type(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "integer()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 3, "Should find 3 functions using integer()");
                // Verify they have integer() in inputs
                for module in &detail.items {
                    for func in &module.entries {
                        assert!(
                            func.inputs.contains("integer()"),
                            "integer() should be in inputs: {}",
                            func.inputs
                        );
                    }
                }
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    // =========================================================================
    // Core functionality tests - ByModule mode
    // =========================================================================

    // by_module counts unique functions (name/arity), not total specs
    // get_user/1 has 2 spec entries, but counts as 1 unique function
    // So: get_user/1, get_user/2, list_users/0, create_user/1, find/1 = 5 unique functions
    #[rstest]
    fn test_struct_usage_by_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: None,
            by_module: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::ByModule(ref by_module) => {
                assert_eq!(by_module.total_modules, 1, "user() only in MyApp.Accounts");
                // total_functions counts raw entries (6), total counts unique (5)
                assert_eq!(by_module.total_functions, 6, "6 spec entries with user()");
                assert_eq!(by_module.modules[0].name, "MyApp.Accounts");
                assert_eq!(by_module.modules[0].total, 5, "5 unique functions");
                // user() only appears in returns, not inputs
                assert_eq!(by_module.modules[0].returns_count, 5);
                assert_eq!(by_module.modules[0].accepts_count, 0);
            }
            _ => panic!("Expected ByModule output"),
        }
    }

    #[rstest]
    fn test_struct_usage_by_module_integer(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "integer()".to_string(),
            module: None,
            by_module: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::ByModule(ref by_module) => {
                assert_eq!(by_module.total_functions, 3, "3 functions use integer()");
                // integer() only appears in inputs, not returns
                for module in &by_module.modules {
                    assert_eq!(
                        module.accepts_count, module.total,
                        "All integer() usage should be in inputs"
                    );
                }
            }
            _ => panic!("Expected ByModule output"),
        }
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_struct_usage_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "NonExistentType.t".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert!(detail.items.is_empty(), "Should find no matches");
                assert_eq!(detail.total_items, 0);
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    #[rstest]
    fn test_struct_usage_by_module_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "NonExistentType.t".to_string(),
            module: None,
            by_module: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::ByModule(ref by_module) => {
                assert!(by_module.modules.is_empty(), "Should find no modules");
                assert_eq!(by_module.total_modules, 0);
                assert_eq!(by_module.total_functions, 0);
            }
            _ => panic!("Expected ByModule output"),
        }
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_struct_usage_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 2, "Limit should restrict to 2 results");
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    #[rstest]
    fn test_struct_usage_regex_pattern(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "Ecto".to_string(), // Regex matches Ecto.Queryable.t()
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 1, "Should find 1 spec matching Ecto");
                // Verify it's the all/1 function
                let func = &detail.items[0].entries[0];
                assert_eq!(func.name, "all");
                assert!(func.inputs.contains("Ecto"));
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    // String.t() appears in 2 specs
    #[rstest]
    fn test_struct_usage_string_type(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "String.t()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 2, "Should find 2 specs with String.t()");
                for module in &detail.items {
                    for func in &module.entries {
                        assert!(
                            func.inputs.contains("String.t()"),
                            "String.t() should be in inputs"
                        );
                    }
                }
            }
            _ => panic!("Expected Detailed output"),
        }
    }

    // Empty pattern returns all 12 specs
    #[rstest]
    fn test_struct_usage_empty_pattern(populated_db: Box<dyn db::backend::Database>) {
        let cmd = StructUsageCmd {
            pattern: "".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        match result {
            StructUsageOutput::Detailed(ref detail) => {
                assert_eq!(detail.total_items, 12, "Empty pattern should return all 12 specs");
            }
            _ => panic!("Expected Detailed output"),
        }
    }
}
