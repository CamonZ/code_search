//! Execute tests for accepts command.

#[cfg(test)]
mod tests {
    use super::super::AcceptsCmd;
    use crate::commands::{CommandRunner, CommonArgs, Execute};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn db::backend::Database> {
        db::test_utils::surreal_accepts_db()
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // The surreal_accepts_db fixture has 9 specs:
    //   MyApp.Accounts: get_user/1(integer()), get_user/2(integer(), keyword()),
    //                   list_users/0(), create_user/1(map())
    //   MyApp.Users:    get_by_email/1(String.t()), authenticate/2(String.t(), String.t())
    //   MyApp.Repo:     get/2(module(), integer()), all/1(Ecto.Queryable.t()),
    //                   insert/2(struct(), keyword())

    #[rstest]
    fn test_accepts_finds_integer_type(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1, get_user/2, and get/2 all accept integer()
        assert_eq!(result.total_items, 3, "Expected 3 functions accepting integer()");

        // Verify the entries contain the right functions
        let mut found: Vec<(String, i64)> = Vec::new();
        for module_group in &result.items {
            for entry in &module_group.entries {
                found.push((entry.name.clone(), entry.arity));
                assert!(
                    entry.inputs.contains("integer()"),
                    "All matched entries should have integer() in inputs, got: {}",
                    entry.inputs
                );
            }
        }
        found.sort();
        assert_eq!(
            found,
            vec![
                ("get".to_string(), 2),
                ("get_user".to_string(), 1),
                ("get_user".to_string(), 2),
            ]
        );
    }

    #[rstest]
    fn test_accepts_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Only get_user/1 and get_user/2 from MyApp.Accounts accept integer()
        assert_eq!(result.total_items, 2, "Expected 2 functions in MyApp.Accounts accepting integer()");

        // Verify all results are from the correct module
        for module_group in &result.items {
            assert_eq!(module_group.name, "MyApp.Accounts");
            for entry in &module_group.entries {
                assert!(entry.inputs.contains("integer()"));
            }
        }

        // Verify module_pattern is set correctly
        assert_eq!(result.module_pattern, "MyApp.Accounts");
    }

    #[rstest]
    fn test_accepts_function_pattern_is_set(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.function_pattern,
            Some("integer()".to_string()),
            "function_pattern should contain the search pattern"
        );
    }

    #[rstest]
    fn test_accepts_module_pattern_defaults_to_star(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.module_pattern, "*", "module_pattern should default to '*' when no module filter");
    }

    #[rstest]
    fn test_accepts_entry_fields_populated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "String.t()".to_string(),
            module: Some("MyApp.Users".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 2, "Expected 2 functions accepting String.t() in MyApp.Users");

        // Verify all field values are populated for each entry
        for module_group in &result.items {
            for entry in &module_group.entries {
                assert!(!entry.name.is_empty(), "name should not be empty");
                assert!(entry.arity > 0, "arity should be positive for these functions");
                assert!(!entry.inputs.is_empty(), "inputs should not be empty");
                assert!(!entry.return_type.is_empty(), "return_type should not be empty");
                assert!(entry.line > 0, "line should be positive");
            }
        }
    }

    // =========================================================================
    // Arity filtering (zero-arity functions excluded from type searches)
    // =========================================================================

    #[rstest]
    fn test_accepts_zero_arity_excluded(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // list_users/0 has no inputs, should not appear in integer() search
        for module_group in &result.items {
            for entry in &module_group.entries {
                assert_ne!(
                    (entry.name.as_str(), entry.arity),
                    ("list_users", 0),
                    "list_users/0 should not match integer() since it has no inputs"
                );
            }
        }
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_accepts_no_match,
        fixture: populated_db,
        cmd: AcceptsCmd {
            pattern: "NonExistentType".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // Limit tests
    // =========================================================================

    #[rstest]
    fn test_accepts_respects_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 2, "Limit of 2 should be respected");
    }

    // =========================================================================
    // Module grouping tests
    // =========================================================================

    #[rstest]
    fn test_accepts_groups_by_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All 9 specs should be returned
        assert_eq!(result.total_items, 9, "Expected all 9 specs");

        // Should be grouped into 3 modules
        let module_names: Vec<&str> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(module_names.contains(&"MyApp.Accounts"), "Should contain MyApp.Accounts");
        assert!(module_names.contains(&"MyApp.Users"), "Should contain MyApp.Users");
        assert!(module_names.contains(&"MyApp.Repo"), "Should contain MyApp.Repo");
    }

    // =========================================================================
    // run() integration test (kills mod.rs:34 mutant)
    // =========================================================================

    #[rstest]
    fn test_run_table_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Verify the output contains meaningful content from all formatting methods
        assert!(
            output.contains("Functions accepting \"integer()\""),
            "run() output should contain the header"
        );
        assert!(
            output.contains("Found 3 function(s)"),
            "run() output should contain the summary"
        );
        assert!(
            output.contains("get_user/1"),
            "run() output should contain formatted entries"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = AcceptsCmd {
            pattern: "integer()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["total_items"], 3);
        assert_eq!(parsed["function_pattern"], "integer()");
    }
}
