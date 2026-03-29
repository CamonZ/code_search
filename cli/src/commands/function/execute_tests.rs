//! Execute tests for function command.

#[cfg(test)]
mod tests {
    use super::super::FunctionCmd;
    use crate::commands::{CommandRunner, CommonArgs, Execute};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: type_signatures,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // type_signatures fixture has these function_locations:
    //   MyApp.Accounts: get_user/1, get_user/2, list_users/0, create_user/1
    //   MyApp.Users:    get_by_email/1, authenticate/2
    //   MyApp.Repo:     get/2, all/1, insert/2

    #[rstest]
    fn test_lookup_returns_correct_function(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user exists at arity 1 and 2 in MyApp.Accounts
        assert_eq!(result.total_items, 2, "Should find both get_user arities");
        assert_eq!(result.items.len(), 1, "Should be grouped into 1 module");
        assert_eq!(result.items[0].name, "MyApp.Accounts");

        let entries = &result.items[0].entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "get_user");
        assert_eq!(entries[0].arity, 1);
        assert_eq!(entries[1].name, "get_user");
        assert_eq!(entries[1].arity, 2);
    }

    #[rstest]
    fn test_lookup_returns_args_and_return_type_fields(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "create_user".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        let entry = &result.items[0].entries[0];
        assert_eq!(entry.name, "create_user");
        assert_eq!(entry.arity, 1);
        // args and return_type are returned as String fields (may be empty
        // because the functions table does not store them; that's fine --
        // we verify the fields are accessible and are strings).
        let _ = entry.args.as_str();
        let _ = entry.return_type.as_str();
    }

    // =========================================================================
    // Arity filtering
    // =========================================================================

    #[rstest]
    fn test_arity_filter_selects_correct_overload(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1, "Arity filter should select exactly one overload");
        let entry = &result.items[0].entries[0];
        assert_eq!(entry.name, "get_user");
        assert_eq!(entry.arity, 1);
    }

    #[rstest]
    fn test_arity_filter_selects_other_overload(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: Some(2),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        assert_eq!(result.items[0].entries[0].arity, 2);
    }

    #[rstest]
    fn test_arity_filter_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: Some(99),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 0, "Non-existent arity should return no results");
        assert!(result.items.is_empty());
    }

    #[rstest]
    fn test_arity_zero(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "list_users".to_string(),
            arity: Some(0),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        assert_eq!(result.items[0].entries[0].name, "list_users");
        assert_eq!(result.items[0].entries[0].arity, 0);
    }

    // =========================================================================
    // Module filtering / scoping
    // =========================================================================

    #[rstest]
    fn test_module_scoping(populated_db: Box<dyn db::backend::Database>) {
        // get/2 exists only in MyApp.Repo, not in MyApp.Accounts
        let cmd = FunctionCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 1);
        assert_eq!(result.items[0].name, "MyApp.Repo");
        assert_eq!(result.items[0].entries[0].name, "get");
        assert_eq!(result.items[0].entries[0].arity, 2);
    }

    #[rstest]
    fn test_module_scoping_wrong_module(populated_db: Box<dyn db::backend::Database>) {
        // get/2 exists in MyApp.Repo, searching in MyApp.Accounts should find nothing
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(result.items.is_empty(), "get should not exist in MyApp.Accounts");
    }

    #[rstest]
    fn test_module_pattern_recorded(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Users".to_string(),
            function: "authenticate".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.module_pattern, "MyApp.Users");
        assert_eq!(result.function_pattern, Some("authenticate".to_string()));
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_no_match_function,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "nonexistent".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    crate::execute_no_match_test! {
        test_name: test_no_match_module,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "NonExistent.Module".to_string(),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // run() integration tests (kills mod.rs:40 mutants)
    // =========================================================================

    /// Tests that run() produces non-empty, correct table output.
    /// Kills: mod.rs:40 run() -> Ok(String::new()) and Ok("xyzzy")
    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Must not be empty (kills -> Ok(String::new()))
        assert!(!output.is_empty(), "run() should produce non-empty output");
        // Must contain expected header (kills -> Ok("xyzzy"))
        assert!(
            output.contains("Function: MyApp.Accounts.get_user"),
            "run() output should contain formatted header, got: {}",
            output
        );
        assert!(
            output.contains("get_user/1"),
            "run() output should contain function entry"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
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
        assert_eq!(parsed["total_items"], 2);
        assert_eq!(parsed["module_pattern"], "MyApp.Accounts");
        assert_eq!(parsed["function_pattern"], "get_user");
    }
}
