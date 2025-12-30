//! Execute tests for path command.

#[cfg(test)]
mod tests {
    use super::super::PathCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Controller.index/2 -> Accounts.list_users/0 (direct call)
    crate::execute_test! {
        test_name: test_path_direct_call,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: 2,
            to_module: "MyApp.Accounts".to_string(),
            to_function: "list_users".to_string(),
            to_arity: 0,
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 1);
            assert_eq!(result.paths[0].steps.len(), 1);
            assert_eq!(result.paths[0].steps[0].caller_module, "MyApp.Controller");
            assert_eq!(result.paths[0].steps[0].callee_module, "MyApp.Accounts");
            assert_eq!(result.paths[0].steps[0].callee_arity, 0);
        },
    }

    // Controller.index/2 -> Accounts.list_users/0 -> Repo.all/1 (2 hops)
    crate::execute_test! {
        test_name: test_path_two_hops,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: 2,
            to_module: "MyApp.Repo".to_string(),
            to_function: "all".to_string(),
            to_arity: 1,
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 1, "Should find exactly 1 path");
            assert_eq!(result.paths[0].steps.len(), 2, "Should have 2 steps");
            // Verify the path: Controller.index/2 -> Accounts.list_users/0 -> Repo.all/1
            // Caller function may have arity suffix from fixture (e.g., "index/2")
            assert!(result.paths[0].steps[0].caller_function.starts_with("index"), "First step caller should start with index");
            assert_eq!(result.paths[0].steps[0].callee_function, "list_users");
            assert_eq!(result.paths[0].steps[0].callee_arity, 0);
            assert!(result.paths[0].steps[1].caller_function.starts_with("list_users"), "Second step caller should start with list_users");
            assert_eq!(result.paths[0].steps[1].callee_function, "all");
            assert_eq!(result.paths[0].steps[1].callee_arity, 1);
        },
    }

    // Controller.show/2 -> Accounts.get_user/1 -> Repo.get/2 (2 hops)
    // show/2 calls get_user/1 which calls get/2
    crate::execute_test! {
        test_name: test_path_via_accounts,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "show".to_string(),
            from_arity: 2,
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            to_arity: 2,
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            // Both get_user/1 and get_user/2 can call Repo.get/2, so there may be multiple paths
            assert!(!result.paths.is_empty(), "Should find at least one path from show/2 to get/2");
            assert!(result.paths.iter().all(|p| p.steps.len() == 2), "All paths should have 2 steps");
            assert!(result.paths[0].steps[0].caller_function.starts_with("show"), "First step caller should start with show");
            assert_eq!(result.paths[0].steps[0].callee_function, "get_user");
            // Should call get_user with some arity
            assert!(result.paths[0].steps[0].callee_arity >= 1, "Should call get_user with arity >= 1");
        },
    }

    // =========================================================================
    // Arity filtering tests
    // =========================================================================

    // Controller.show/2 -> Accounts.get_user/2 -> Repo.get/2 (with from_arity)
    crate::execute_test! {
        test_name: test_path_with_from_arity,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "show".to_string(),
            from_arity: 2,
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            to_arity: 2,
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            // Should find path from show/2 to get/2
            assert!(!result.paths.is_empty(), "Should find at least one path");
            // First step caller should start with show
            assert!(result.paths[0].steps[0].caller_function.starts_with("show"), "First step caller should start with show");
        },
    }

    // Controller.index/2 -> Accounts.list_users/0 (with from_arity exact match)
    crate::execute_test! {
        test_name: test_path_with_from_arity_exact,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: 2,
            to_module: "MyApp.Accounts".to_string(),
            to_function: "list_users".to_string(),
            to_arity: 0,
            depth: 10,
            limit: 10,
        },
        assertions: |result| {
            assert_eq!(result.paths.len(), 1, "Should find exactly 1 path");
            // Caller function may have arity suffix from fixture (e.g., "index/2")
            assert!(result.paths[0].steps[0].caller_function.starts_with("index"), "Caller function should start with index");
            assert_eq!(result.paths[0].steps[0].callee_arity, 0);
        },
    }

    // Wrong arity should find no paths
    crate::execute_no_match_test! {
        test_name: test_path_with_wrong_from_arity,
        fixture: populated_db,
        cmd: PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: 99,  // Wrong arity - index is /2
            to_module: "MyApp.Accounts".to_string(),
            to_function: "list_users".to_string(),
            to_arity: 0,
            depth: 10,
            limit: 10,
        },
        empty_field: paths,
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
            from_arity: 2,
            to_module: "MyApp.Controller".to_string(),
            to_function: "index".to_string(),
            to_arity: 2,
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
            from_arity: 2,
            to_module: "MyApp.Repo".to_string(),
            to_function: "all".to_string(),
            to_arity: 1,
            depth: 1,
            limit: 10,
        },
        empty_field: paths,
    }

}
