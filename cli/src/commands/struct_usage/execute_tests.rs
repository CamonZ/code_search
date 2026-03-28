//! Execute tests for struct-usage command.

#[cfg(test)]
mod tests {
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

    // =========================================================================
    // Sort order tests - kills mutant: replace == with != in build_struct_modules_result
    // =========================================================================

    // This test calls build_struct_modules_result directly with synthetic data
    // designed so the mutant (== to !=) produces a detectably wrong order.
    //
    // With the mutant, the sort comparator applies alphabetical ordering when
    // totals DIFFER (instead of when they're equal). So a module with a higher
    // total but later alphabetical position would be sorted wrong.
    //
    // Setup:
    //   "Zebra" has 3 functions (highest total, alphabetically last)
    //   "Alpha" has 2 functions (tied, alphabetically first)
    //   "Beta"  has 2 functions (tied, alphabetically second)
    //
    // Correct order (total desc, then alpha): Zebra(3), Alpha(2), Beta(2)
    // Mutant order  (alpha when !=, stable when ==): Alpha(2), Beta(2), Zebra(3)
    #[test]
    fn test_by_module_equal_totals_sort_alphabetically() {
        use db::queries::struct_usage::StructUsageEntry;
        use super::super::execute::build_struct_modules_result;

        let entries = vec![
            // 3 unique functions in "Zebra" (highest total, alphabetically last)
            StructUsageEntry {
                project: "test".into(),
                module: "Zebra".into(),
                name: "z_func1".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 1,
            },
            StructUsageEntry {
                project: "test".into(),
                module: "Zebra".into(),
                name: "z_func2".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 2,
            },
            StructUsageEntry {
                project: "test".into(),
                module: "Zebra".into(),
                name: "z_func3".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 3,
            },
            // 2 unique functions in "Alpha" (tied with Beta, alphabetically first)
            StructUsageEntry {
                project: "test".into(),
                module: "Alpha".into(),
                name: "a_func1".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 10,
            },
            StructUsageEntry {
                project: "test".into(),
                module: "Alpha".into(),
                name: "a_func2".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 11,
            },
            // 2 unique functions in "Beta" (tied with Alpha, alphabetically second)
            StructUsageEntry {
                project: "test".into(),
                module: "Beta".into(),
                name: "b_func1".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 20,
            },
            StructUsageEntry {
                project: "test".into(),
                module: "Beta".into(),
                name: "b_func2".into(),
                arity: 1,
                inputs_string: "t()".into(),
                return_string: "t()".into(),
                line: 21,
            },
        ];

        let result = build_struct_modules_result("t()".into(), entries);

        assert_eq!(result.modules.len(), 3, "Should have 3 modules");

        // Zebra has the highest total (3), must be first despite being last alphabetically
        assert_eq!(result.modules[0].name, "Zebra");
        assert_eq!(result.modules[0].total, 3);

        // Alpha and Beta are tied at 2; alphabetical tiebreaker puts Alpha before Beta
        assert_eq!(result.modules[1].name, "Alpha");
        assert_eq!(result.modules[1].total, 2);
        assert_eq!(result.modules[2].name, "Beta");
        assert_eq!(result.modules[2].total, 2);
    }

    // =========================================================================
    // Integration test through run() — asserts on formatted output
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: None,
            by_module: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };

        let output = cmd.run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Verify the output contains the expected header and module
        assert!(output.contains("Functions using \"user()\""), "Should contain header");
        assert!(output.contains("MyApp.Accounts:"), "Should contain module name");
        assert!(output.contains("get_user/1"), "Should contain function name");
    }

    #[rstest]
    fn test_run_by_module_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = StructUsageCmd {
            pattern: "user()".to_string(),
            module: None,
            by_module: true,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };

        let output = cmd.run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Verify the output contains the expected header and table structure
        assert!(output.contains("Modules using \"user()\""), "Should contain header");
        assert!(output.contains("MyApp.Accounts"), "Should contain module name");
        assert!(output.contains("Accepts"), "Should contain table header");
        assert!(output.contains("Returns"), "Should contain table header");
    }
}
