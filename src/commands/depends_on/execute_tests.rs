//! Execute tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::DependsOnCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Controller depends on: Accounts (2 calls: list_users, get_user) and Service (1 call: process)
    crate::execute_test! {
        test_name: test_depends_on_single_module,
        fixture: populated_db,
        cmd: DependsOnCmd {
            module: "MyApp.Controller".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.dependencies.len(), 2);
            assert!(result.dependencies.iter().any(|d| d.module == "MyApp.Accounts"));
            assert!(result.dependencies.iter().any(|d| d.module == "MyApp.Service"));
        },
    }

    // Service depends on: Repo (1 call via do_fetch) and Notifier (1 call via process)
    // Self-calls (process→fetch, fetch→do_fetch) are excluded
    crate::execute_test! {
        test_name: test_depends_on_counts_calls,
        fixture: populated_db,
        cmd: DependsOnCmd {
            module: "MyApp.Service".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.dependencies.len(), 2);
            assert!(result.dependencies.iter().any(|d| d.module == "MyApp.Repo"));
            assert!(result.dependencies.iter().any(|d| d.module == "MyApp.Notifier"));
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_depends_on_no_match,
        fixture: populated_db,
        cmd: DependsOnCmd {
            module: "NonExistent".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: dependencies,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_depends_on_excludes_self,
        fixture: populated_db,
        cmd: DependsOnCmd {
            module: "MyApp.Repo".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        collection: dependencies,
        condition: |d| d.module != "MyApp.Repo",
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: DependsOnCmd,
        cmd: DependsOnCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
