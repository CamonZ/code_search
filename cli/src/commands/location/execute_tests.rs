//! Execute tests for location command.

#[cfg(test)]
mod tests {
    use super::super::LocationCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // The complex fixture has clauses:
    // - Accounts: get_user/1 at line 10, get_user/2 at line 18, list_users/0 at line 24,
    //             notify_change/1 at line 40, validate_email/1 at line 30, format_name/1 at line 36,
    //             __struct__/0 at line 1, __generated__/0 at line 45
    // - Controller: index/2 at line 5, show/2 at line 12, create/2 at lines 25 and 28,
    //               handle_event/1 at line 35, format_display/1 at line 42, __generated__/0 at line 50
    // - Service: process_request/2 at lines 8, 12, 16 (3 clauses), transform_data/1 at line 22,
    //            get_context/1 at line 28, validate/1 at line 32
    // - Repo: get/2 at line 10, all/1 at line 15, insert/1 at line 20, query/2 at line 28,
    //         validate/1 at line 35

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_location_exact_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "MyApp.Accounts");
        assert_eq!(result.modules[0].functions.len(), 1);

        let func = &result.modules[0].functions[0];
        assert_eq!(func.name, "get_user");
        assert_eq!(func.arity, 1);
        assert_eq!(func.file, "lib/my_app/accounts.ex");
        assert_eq!(func.clauses[0].start_line, 10);
    }

    // get_user/1 has 2 clauses (lines 10, 12), get_user/2 has 1 clause (line 17) = 3 total
    #[rstest]
    fn test_location_without_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 3, "get_user/1 has 2 clauses + get_user/2 has 1");
        assert_eq!(result.modules[0].functions.len(), 2, "Two function arities");
    }

    // process_request/2 has 3 clauses at lines 8, 12, 16
    #[rstest]
    fn test_location_multiple_clauses(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Service".to_string()),
            function: "process_request".to_string(),
            arity: Some(2),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].functions.len(), 1);

        let func = &result.modules[0].functions[0];
        assert_eq!(func.clauses.len(), 3, "process_request/2 has 3 clauses");

        // Verify clause lines
        let lines: Vec<i64> = func.clauses.iter().map(|c| c.start_line).collect();
        assert!(lines.contains(&8), "Should have clause at line 8");
        assert!(lines.contains(&12), "Should have clause at line 12");
        assert!(lines.contains(&16), "Should have clause at line 16");
    }

    #[rstest]
    fn test_location_without_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1 has 2 clauses, get_user/2 has 1 clause = 3 total
        assert_eq!(result.total_clauses, 3);
        assert_eq!(result.modules.len(), 1);
        assert_eq!(result.modules[0].name, "MyApp.Accounts");
    }

    // =========================================================================
    // Regex tests
    // =========================================================================

    #[rstest]
    fn test_location_with_regex(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp\\..*".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // get_user/1 (2 clauses) + get_user/2 (1 clause) + list_users/0 (1 clause) = 4
        assert_eq!(result.total_clauses, 4);
    }

    // validate exists in multiple modules (Service, Repo)
    #[rstest]
    fn test_location_function_across_modules(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "validate".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // validate/1 exists in both Service and Repo
        assert_eq!(result.total_clauses, 2, "validate/1 in Service and Repo");
        assert_eq!(result.modules.len(), 2, "Should be in 2 modules");
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_location_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("NonExistent".to_string()),
            function: "foo".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.modules.is_empty());
        assert_eq!(result.total_clauses, 0);
    }

    #[rstest]
    fn test_location_wrong_arity(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(99), // Non-existent arity
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.modules.is_empty());
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_location_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: ".*".to_string(),
            arity: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 3,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 3, "Limit should restrict to 3 clauses");
    }

    #[rstest]
    fn test_location_arity_zero(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: None,
            function: "list_users".to_string(),
            arity: Some(0),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clauses, 1);
        assert_eq!(result.modules[0].functions[0].arity, 0);
    }

    // =========================================================================
    // Output format tests
    // =========================================================================

    #[rstest]
    fn test_location_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(!result.modules.is_empty(), "Should find at least one module");
        let func = &result.modules[0].functions[0];
        // Verify we can construct a file:line format
        assert!(func.file.ends_with(".ex"), "File should be .ex: {}", func.file);
        assert!(func.clauses[0].start_line > 0);
    }
}
