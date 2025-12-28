//! Execute tests for depended-by command.

/// CozoDB tests use JSON-based fixtures
#[cfg(all(test, not(feature = "backend-surrealdb")))]
mod tests {
    use super::super::DependedByCmd;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // MyApp.Repo is depended on by: Accounts (3 calls), Service (1 call via do_fetch)
    crate::execute_test! {
        test_name: test_depended_by_single_module,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.items.len(), 2);
            assert!(result.items.iter().any(|m| m.name == "MyApp.Accounts"));
            assert!(result.items.iter().any(|m| m.name == "MyApp.Service"));
        },
    }

    crate::execute_test! {
        test_name: test_depended_by_counts_calls,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Accounts has 3 callers, Service has 1
            let accounts = result.items.iter().find(|m| m.name == "MyApp.Accounts").unwrap();
            let service = result.items.iter().find(|m| m.name == "MyApp.Service").unwrap();
            let accounts_calls: usize = accounts.entries.iter().map(|c| c.targets.len()).sum();
            let service_calls: usize = service.entries.iter().map(|c| c.targets.len()).sum();
            assert_eq!(accounts_calls, 3);
            assert_eq!(service_calls, 1);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_depended_by_no_match,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "NonExistent".to_string(),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: items,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_depended_by_excludes_self,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        collection: items,
        condition: |m| m.name != "MyApp.Repo",
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: DependedByCmd,
        cmd: DependedByCmd {
            module: "MyApp".to_string(),
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
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
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
                project: "test_project".to_string(),
                regex: false,
                limit: 1,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 1, "Limit should restrict to 1 call");
    }
}
