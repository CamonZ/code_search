//! Execute tests for depended-by command.

#[cfg(test)]
mod tests {
    use super::super::DependedByCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::collections::HashSet;

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // MyApp.Repo is depended on by 2 modules with 3 total calls:
    // - Accounts.get_user/1 → Repo.get/2
    // - Accounts.list_users/0 → Repo.all/1
    // - Logger.log_query/2 → Repo.insert/1
    #[rstest]
    fn test_depended_by_single_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.module_pattern, "MyApp.Repo", "module_pattern should match input");
        assert_eq!(result.total_items, 3, "Should have 3 total calls");
        assert_eq!(result.items.len(), 2, "Should have 2 dependent modules");

        let module_names: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(
            module_names.contains("MyApp.Accounts"),
            "Should include MyApp.Accounts"
        );
        assert!(
            module_names.contains("MyApp.Logger"),
            "Should include MyApp.Logger"
        );
    }

    #[rstest]
    fn test_depended_by_counts_calls(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Accounts has 2 callers, Logger has 1
        let accounts = result
            .items
            .iter()
            .find(|m| m.name == "MyApp.Accounts")
            .expect("Should find Accounts module");
        let logger = result
            .items
            .iter()
            .find(|m| m.name == "MyApp.Logger")
            .expect("Should find Logger module");

        let accounts_calls: usize = accounts.entries.iter().map(|c| c.targets.len()).sum();
        let logger_calls: usize = logger.entries.iter().map(|c| c.targets.len()).sum();

        assert_eq!(accounts_calls, 2, "Accounts should have 2 calls to Repo");
        assert_eq!(logger_calls, 1, "Logger should have 1 call to Repo");
    }

    #[rstest]
    fn test_depended_by_caller_fields(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Logger has one caller: log_query/2
        let logger = result
            .items
            .iter()
            .find(|m| m.name == "MyApp.Logger")
            .expect("Should find Logger module");

        assert_eq!(logger.entries.len(), 1, "Logger should have 1 caller function");
        let caller = &logger.entries[0];
        assert_eq!(caller.function, "log_query", "Caller function name should be log_query");
        assert_eq!(caller.arity, 2, "Caller arity should be 2");

        // Verify target details
        assert_eq!(caller.targets.len(), 1, "log_query should have 1 target");
        let target = &caller.targets[0];
        assert_eq!(target.function, "insert", "Target function should be insert");
        assert_eq!(target.arity, 1, "Target arity should be 1");
        assert!(target.line > 0, "Target call line should be positive");
    }

    #[rstest]
    fn test_depended_by_function_pattern_is_none(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(result.function_pattern.is_none(), "function_pattern should be None");
    }

    // MyApp.Accounts is depended on by 3 modules with 4 calls (excluding self-reference):
    // - Controller.index/2 → list_users/0
    // - Controller.show/2 → get_user/2
    // - Service.process_request/2 → get_user/1
    // - Cache.invalidate/1 → notify_change/1
    #[rstest]
    fn test_depended_by_accounts(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Accounts".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.items.len(), 3, "Should have 3 dependent modules");

        let module_names: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(
            module_names.contains("MyApp.Controller"),
            "Should include Controller"
        );
        assert!(
            module_names.contains("MyApp.Service"),
            "Should include Service"
        );
        assert!(module_names.contains("MyApp.Cache"), "Should include Cache");

        // Self-reference should be excluded
        assert!(
            !module_names.contains("MyApp.Accounts"),
            "Self-reference should be excluded"
        );
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_depended_by_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "NonExistent".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no modules for non-existent target"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_depended_by_excludes_self(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Accounts".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All dependent modules should NOT be MyApp.Accounts (the target)
        for module in &result.items {
            assert_ne!(
                module.name, "MyApp.Accounts",
                "Self-references should be excluded"
            );
        }
    }

    #[rstest]
    fn test_depended_by_with_regex(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp\\.Repo".to_string(),
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Should find dependents of MyApp.Repo
        assert!(!result.items.is_empty(), "Should find dependents with regex");
        assert_eq!(result.items.len(), 2, "Should have 2 dependent modules");
    }

    #[rstest]
    fn test_depended_by_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependedByCmd {
            module: "MyApp.Accounts".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 1,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 1, "Limit should restrict to 1 call");
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(
            output.contains("Modules that depend on: MyApp.Repo"),
            "Table output should contain header"
        );
        assert!(
            output.contains("MyApp.Accounts"),
            "Table should contain dependent module"
        );
        assert!(
            output.contains("Found 3 call(s) from 2 module(s):"),
            "Table should contain summary"
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DependedByCmd {
            module: "NonExistent".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(
            output.contains("Modules that depend on: NonExistent"),
            "Header should contain queried module"
        );
        assert!(
            output.contains("No dependents found."),
            "Empty result should show empty message"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Repo");
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["total_items"], 3);
    }
}
