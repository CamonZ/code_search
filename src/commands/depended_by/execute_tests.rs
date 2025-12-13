//! Execute tests for depended-by command.

#[cfg(test)]
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
