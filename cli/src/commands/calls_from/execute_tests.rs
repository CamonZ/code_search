//! Execute tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::CallsFromCmd;
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

    // MyApp.Accounts has 4 calls in the complex fixture:
    // - get_user/1 → MyApp.Repo.get/2
    // - get_user/2 → MyApp.Accounts.get_user/1
    // - list_users/0 → MyApp.Repo.all/1
    // - notify_change/1 → MyApp.Controller.handle_event/1
    #[rstest]
    fn test_calls_from_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 4, "Expected 4 calls from MyApp.Accounts");

        // Collect all calls as (caller_name, caller_arity, callee_module, callee_name, callee_arity)
        let mut actual_calls: HashSet<(String, i64, String, String, i64)> = HashSet::new();
        for module_group in &result.items {
            for func in &module_group.entries {
                for call in &func.calls {
                    actual_calls.insert((
                        func.name.clone(),
                        func.arity,
                        call.callee.module.to_string(),
                        call.callee.name.to_string(),
                        call.callee.arity,
                    ));
                }
            }
        }

        // Verify expected calls
        assert!(
            actual_calls.contains(&("get_user".to_string(), 1, "MyApp.Repo".to_string(), "get".to_string(), 2)),
            "Should contain get_user/1 → Repo.get/2"
        );
        assert!(
            actual_calls.contains(&("get_user".to_string(), 2, "MyApp.Accounts".to_string(), "get_user".to_string(), 1)),
            "Should contain get_user/2 → get_user/1"
        );
        assert!(
            actual_calls.contains(&("list_users".to_string(), 0, "MyApp.Repo".to_string(), "all".to_string(), 1)),
            "Should contain list_users/0 → Repo.all/1"
        );
        assert!(
            actual_calls.contains(&("notify_change".to_string(), 1, "MyApp.Controller".to_string(), "handle_event".to_string(), 1)),
            "Should contain notify_change/1 → Controller.handle_event/1"
        );
    }

    // get_user functions: get_user/1→Repo.get, get_user/2→get_user/1 = 2 calls
    #[rstest]
    fn test_calls_from_function(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 2, "Expected 2 calls from get_user functions");

        // Verify both get_user variants are present
        let mut found_get_user_1 = false;
        let mut found_get_user_2 = false;
        for module_group in &result.items {
            for func in &module_group.entries {
                assert_eq!(func.name, "get_user", "All functions should be get_user");
                if func.arity == 1 {
                    found_get_user_1 = true;
                    assert_eq!(func.calls.len(), 1);
                    assert_eq!(func.calls[0].callee.module.as_ref(), "MyApp.Repo");
                    assert_eq!(func.calls[0].callee.name.as_ref(), "get");
                } else if func.arity == 2 {
                    found_get_user_2 = true;
                    assert_eq!(func.calls.len(), 1);
                    assert_eq!(func.calls[0].callee.module.as_ref(), "MyApp.Accounts");
                    assert_eq!(func.calls[0].callee.name.as_ref(), "get_user");
                }
            }
        }
        assert!(found_get_user_1, "Should find get_user/1");
        assert!(found_get_user_2, "Should find get_user/2");
    }

    // All calls from MyApp.* modules - there are 24 calls in the complex fixture
    #[rstest]
    fn test_calls_from_regex_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // The complex fixture has 24 calls total from MyApp.* modules
        assert_eq!(result.total_items, 24, "Expected 24 calls from MyApp.* modules");

        // Verify we have calls from multiple modules
        let modules: HashSet<_> = result.items.iter().map(|m| m.name.as_str()).collect();
        assert!(modules.contains("MyApp.Accounts"), "Should include MyApp.Accounts");
        assert!(modules.contains("MyApp.Controller"), "Should include MyApp.Controller");
        assert!(modules.contains("MyApp.Service"), "Should include MyApp.Service");
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_calls_from_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Expected no modules for non-existent module"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_calls_from_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            common: CommonArgs {
                regex: true,
                limit: 1,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 1, "Limit should restrict to 1 call");
    }
}
