//! Execute tests for function command.

#[cfg(test)]
mod tests {
    use super::super::FunctionCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: type_signatures,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // MyApp.Accounts has 2 get_user functions (arity 1 and 2)
    crate::execute_test! {
        test_name: test_function_exact_match,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_functions, 2);
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].functions.len(), 2);
        },
    }

    crate::execute_test! {
        test_name: test_function_with_arity,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_functions, 1);
            let func = &result.modules[0].functions[0];
            assert_eq!(func.arity, 1);
            assert_eq!(func.args, "integer()");
            assert_eq!(func.return_type, "User.t() | nil");
        },
    }

    // Functions containing "user": get_user/1, get_user/2, list_users, create_user = 4
    crate::execute_test! {
        test_name: test_function_regex_match,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp\\..*".to_string(),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.total_functions, 4);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_function_no_match,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: modules,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_function_with_project_filter,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].name, "MyApp.Accounts");
        },
    }

    crate::execute_test! {
        test_name: test_function_with_limit,
        fixture: populated_db,
        cmd: FunctionCmd {
            module: "MyApp\\..*".to_string(),
            function: ".*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 2,
        },
        assertions: |result| {
            // Limit applies to raw results before grouping
            assert_eq!(result.total_functions, 2);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: FunctionCmd,
        cmd: FunctionCmd {
            module: "MyApp".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
