//! Execute tests for depended-by command.

#[cfg(test)]
mod tests {
    use super::super::DependedByCmd;
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.dependents.len(), 2);
            assert!(result.dependents.iter().any(|d| d.module == "MyApp.Accounts"));
            assert!(result.dependents.iter().any(|d| d.module == "MyApp.Service"));
        },
    }

    crate::execute_test! {
        test_name: test_depended_by_counts_calls,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            let accounts = result.dependents.iter().find(|d| d.module == "MyApp.Accounts").unwrap();
            let service = result.dependents.iter().find(|d| d.module == "MyApp.Service").unwrap();
            assert_eq!(accounts.call_count, 3);
            assert_eq!(service.call_count, 1);
        },
    }

    // Ordered by count descending: Accounts (3) before Service (1)
    crate::execute_test! {
        test_name: test_depended_by_ordered_by_count,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.dependents[0].module, "MyApp.Accounts");
            assert_eq!(result.dependents[1].module, "MyApp.Service");
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
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: dependents,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_depended_by_excludes_self,
        fixture: populated_db,
        cmd: DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        collection: dependents,
        condition: |d| d.module != "MyApp.Repo",
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: DependedByCmd,
        cmd: DependedByCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
