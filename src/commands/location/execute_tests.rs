//! Execute tests for location command.

#[cfg(test)]
mod tests {
    use super::super::LocationCmd;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_location_exact_match,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 1);
            assert_eq!(result.locations[0].file, "lib/my_app/accounts.ex");
            assert_eq!(result.locations[0].start_line, 10);
            assert_eq!(result.locations[0].end_line, 15);
        },
    }

    // get_user exists in Accounts with arities 1 and 2
    crate::execute_test! {
        test_name: test_location_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 2);
            assert!(result.locations.iter().all(|l| l.module == "MyApp.Accounts"));
        },
    }

    // Functions with "user" in name: get_user/1, get_user/2, list_users = 3
    crate::execute_count_test! {
        test_name: test_location_without_module_multiple_matches,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: locations,
        expected: 3,
    }

    // get_user has two arities in Accounts
    crate::execute_count_test! {
        test_name: test_location_without_arity,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        field: locations,
        expected: 2,
    }

    crate::execute_count_test! {
        test_name: test_location_with_regex,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp\\..*".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: locations,
        expected: 3,
    }

    crate::execute_test! {
        test_name: test_location_format,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations[0].format_location(), "lib/my_app/accounts.ex:10:15");
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_location_no_match,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("NonExistent".to_string()),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: locations,
    }

    crate::execute_no_match_test! {
        test_name: test_location_nonexistent_project,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "nonexistent_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: locations,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_location_with_project_filter,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 1);
            assert_eq!(result.locations[0].project, "test_project");
        },
    }

    // 6 functions with arity 1: get_user/1, validate_email, process, fetch, all, notify
    crate::execute_test! {
        test_name: test_location_arity_filter_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 6);
            assert!(result.locations.iter().all(|l| l.arity == 1));
        },
    }

    crate::execute_test! {
        test_name: test_location_project_filter_without_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 2);
            assert!(result.locations.iter().all(|l| l.project == "test_project"));
        },
    }

    // Accounts has get_user/1, get_user/2, list_users matching ".*user.*" = 3
    crate::execute_count_test! {
        test_name: test_location_function_regex_with_exact_module,
        fixture: populated_db,
        cmd: LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: locations,
        expected: 3,
    }

    crate::execute_test! {
        test_name: test_location_arity_zero,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: "list_users".to_string(),
            arity: Some(0),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.locations.len(), 1);
            assert_eq!(result.locations[0].arity, 0);
        },
    }

    crate::execute_limit_test! {
        test_name: test_location_with_limit,
        fixture: populated_db,
        cmd: LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 1,
        },
        collection: locations,
        limit: 1,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: LocationCmd,
        cmd: LocationCmd {
            module: Some("MyApp".to_string()),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
