//! Execute tests for returns command.

#[cfg(test)]
mod tests {
    use super::super::ReturnsCmd;
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
    //   MyApp.Accounts: get_user/1, get_user/2, list_users/0, create_user/1
    //   MyApp.Users:    get_by_email/1, authenticate/2
    //   MyApp.Repo:     get/2, all/1, insert/2
    //
    // Return types containing "user()":
    //   get_user/1, get_user/2, list_users/0, create_user/1, get_by_email/1

    #[rstest]
    fn test_returns_finds_user_type(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 5, "Expected 5 functions returning user()");

        // Verify all entries contain user() in return_type
        for module_group in &result.items {
            for entry in &module_group.entries {
                assert!(
                    entry.return_type.contains("user()"),
                    "All matched entries should have user() in return_type, got: {}",
                    entry.return_type
                );
            }
        }
    }

    #[rstest]
    fn test_returns_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1, get_user/2, list_users/0, create_user/1 all return user()
        assert_eq!(result.total_items, 4, "Expected 4 functions in MyApp.Accounts returning user()");

        // Verify all results are from the correct module
        for module_group in &result.items {
            assert_eq!(module_group.name, "MyApp.Accounts");
            for entry in &module_group.entries {
                assert!(entry.return_type.contains("user()"));
            }
        }

        // Verify module_pattern is set correctly
        assert_eq!(result.module_pattern, "MyApp.Accounts");
    }

    #[rstest]
    fn test_returns_function_pattern_is_set(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
            module: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.function_pattern,
            Some("user()".to_string()),
            "function_pattern should contain the search pattern"
        );
    }

    #[rstest]
    fn test_returns_module_pattern_defaults_to_star(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
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
    fn test_returns_entry_fields_populated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
            module: Some("MyApp.Accounts".to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Verify all field values are populated for each entry
        for module_group in &result.items {
            for entry in &module_group.entries {
                assert!(!entry.name.is_empty(), "name should not be empty");
                assert!(entry.arity >= 0, "arity should be non-negative");
                assert!(!entry.return_type.is_empty(), "return_type should not be empty");
                assert!(entry.line > 0, "line should be positive");
            }
        }
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_returns_no_match,
        fixture: populated_db,
        cmd: ReturnsCmd {
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
    fn test_returns_respects_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
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
    fn test_returns_groups_by_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
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
    // run() integration tests (kills mod.rs:34 mutant)
    // =========================================================================

    #[rstest]
    fn test_run_table_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
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
            output.contains("Functions returning \"user()\""),
            "run() output should contain the header"
        );
        assert!(
            output.contains("Found 5 function(s)"),
            "run() output should contain the summary"
        );
        assert!(
            output.contains("get_user/1"),
            "run() output should contain formatted entries"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = ReturnsCmd {
            pattern: "user()".to_string(),
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
        assert_eq!(parsed["total_items"], 5);
        assert_eq!(parsed["function_pattern"], "user()");
    }
}
