//! Execute tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::DependsOnCmd;
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

    // MyApp.Controller depends on (outgoing calls):
    // - MyApp.Accounts (Controller.index -> list_users, Controller.show -> get_user)
    // - MyApp.Service (Controller.create -> process_request)
    // - MyApp.Notifier (Controller.create -> send_email)
    // - MyApp.Events (Controller.create -> publish)
    #[rstest]
    fn test_depends_on_controller(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Controller".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.items.len(), 4, "Controller should depend on 4 modules");

        let module_names: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(
            module_names.contains("MyApp.Accounts"),
            "Should include MyApp.Accounts"
        );
        assert!(
            module_names.contains("MyApp.Service"),
            "Should include MyApp.Service"
        );
        assert!(
            module_names.contains("MyApp.Notifier"),
            "Should include MyApp.Notifier"
        );
        assert!(
            module_names.contains("MyApp.Events"),
            "Should include MyApp.Events"
        );
    }

    // MyApp.Service depends on:
    // - MyApp.Accounts (process_request -> get_user/1)
    // - MyApp.Notifier (process_request -> send_email/2)
    // - MyApp.Logger (process_request -> log_query/2)
    #[rstest]
    fn test_depends_on_service(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.items.len(), 3, "Service should depend on 3 modules");

        let module_names: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(
            module_names.contains("MyApp.Accounts"),
            "Should include MyApp.Accounts"
        );
        assert!(
            module_names.contains("MyApp.Notifier"),
            "Should include MyApp.Notifier"
        );
        assert!(
            module_names.contains("MyApp.Logger"),
            "Should include MyApp.Logger"
        );
    }

    #[rstest]
    fn test_depends_on_verifies_dependency_functions(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Find the Accounts dependency and verify the function details
        let accounts = result
            .items
            .iter()
            .find(|m| m.name == "MyApp.Accounts")
            .expect("Should find Accounts dependency");

        // Service calls Accounts.get_user/1
        assert_eq!(accounts.entries.len(), 1, "Should call 1 function in Accounts");
        assert_eq!(accounts.entries[0].name, "get_user");
        assert_eq!(accounts.entries[0].arity, 1);
        assert_eq!(
            accounts.entries[0].callers.len(),
            1,
            "Should have 1 caller"
        );

        // Verify the caller is Service.process_request
        let caller = &accounts.entries[0].callers[0];
        assert_eq!(caller.caller.module.as_ref(), "MyApp.Service");
        assert_eq!(caller.caller.name.as_ref(), "process_request");
    }

    #[rstest]
    fn test_depends_on_total_items(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Service has 3 outgoing calls: to Accounts, Notifier, Logger
        assert_eq!(result.total_items, 3, "Should have 3 total dependency calls");
    }

    #[rstest]
    fn test_depends_on_module_pattern(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.module_pattern, "MyApp.Service",
            "module_pattern should match the queried module"
        );
        assert!(
            result.function_pattern.is_none(),
            "function_pattern should be None"
        );
    }

    #[rstest]
    fn test_depends_on_file_is_empty(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Controller".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Dependency module groups should have empty file (by design)
        for module in &result.items {
            assert!(
                module.file.is_empty(),
                "Module group file should be empty for depends_on, got: '{}'",
                module.file
            );
        }
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_depends_on_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "NonExistent".to_string(),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no modules for non-existent source"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_depends_on_with_regex(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp\\.Service".to_string(),
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Should find dependencies of MyApp.Service
        assert!(!result.items.is_empty(), "Should find dependencies with regex");
        assert_eq!(result.items.len(), 3, "Should have 3 dependency modules");
    }

    #[rstest]
    fn test_depends_on_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = DependsOnCmd {
            module: "MyApp.Controller".to_string(),
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

        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
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
            output.contains("Dependencies of: MyApp.Service"),
            "Table output should contain header"
        );
        assert!(
            output.contains("MyApp.Accounts"),
            "Table should contain dependency module"
        );
        assert!(
            output.contains("Found 3 call(s) to 3 module(s):"),
            "Table should contain summary"
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DependsOnCmd {
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
            output.contains("Dependencies of: NonExistent"),
            "Header should contain queried module"
        );
        assert!(
            output.contains("No dependencies found."),
            "Empty result should show empty message"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
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
        assert_eq!(parsed["module_pattern"], "MyApp.Service");
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["total_items"], 3);
    }
}
