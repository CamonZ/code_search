//! Execute tests for function command.

#[cfg(test)]
mod tests {
    use super::super::execute::FunctionResult;
    use super::super::FunctionCmd;
    use crate::commands::Execute;
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
    crate::execute_count_test! {
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
        field: functions,
        expected: 2,
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
            assert_eq!(result.functions.len(), 1);
            assert_eq!(result.functions[0].arity, 1);
            assert_eq!(result.functions[0].args, "integer()");
            assert_eq!(result.functions[0].return_type, "User.t() | nil");
        },
    }

    // Functions containing "user": get_user/1, get_user/2, list_users, create_user = 4
    crate::execute_count_test! {
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
        field: functions,
        expected: 4,
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
        empty_field: functions,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
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
        collection: functions,
        condition: |f| f.project == "test_project",
    }

    crate::execute_limit_test! {
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
        collection: functions,
        limit: 2,
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
