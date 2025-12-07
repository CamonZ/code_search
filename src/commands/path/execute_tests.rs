//! Execute tests for path command.

#[cfg(test)]
mod tests {
    use super::super::execute::PathResult;
    use super::super::PathCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Controller.index -> Accounts.list_users (direct call)
    crate::execute_test! {
        test_name: test_path_direct_call,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "MyApp.Accounts".to_string(),
            to_function: "list_users".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 1);
            assert_eq!(result.paths[0].steps.len(), 1);
            assert_eq!(result.paths[0].steps[0].caller_module, "MyApp.Controller");
            assert_eq!(result.paths[0].steps[0].callee_module, "MyApp.Accounts");
        },
    }

    // Controller.index -> Accounts.list_users -> Repo.all (2 hops)
    crate::execute_test! {
        test_name: test_path_two_hops,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "MyApp.Repo".to_string(),
            to_function: "all".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 1);
            assert_eq!(result.paths[0].steps.len(), 2);
        },
    }

    // Controller.show -> Accounts.get_user -> Repo.get (2 hops)
    // Both get_user/1 and get_user/2 call Repo.get, so 2 paths found
    crate::execute_test! {
        test_name: test_path_via_accounts,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "show".to_string(),
            from_arity: None,
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 2);
            assert!(result.paths.iter().all(|p| p.steps.len() == 2));
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    // No path from Repo back to Controller (acyclic)
    crate::execute_no_match_test! {
        test_name: test_path_no_path_exists,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Repo".to_string(),
            from_function: "get".to_string(),
            from_arity: None,
            to_module: "MyApp.Controller".to_string(),
            to_function: "index".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        },
        empty_field: paths,
    }

    // Depth 1 can't reach Repo.all from Controller.index (needs 2 hops)
    crate::execute_no_match_test! {
        test_name: test_path_depth_limit,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "MyApp.Repo".to_string(),
            to_function: "all".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 1,
            limit: 10,
        },
        empty_field: paths,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: PathCmd,
        cmd: PathCmd {
            from_module: "MyApp".to_string(),
            from_function: "foo".to_string(),
            from_arity: None,
            to_module: "MyApp".to_string(),
            to_function: "bar".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        },
    }
}
