//! Execute tests for search command.

#[cfg(test)]
mod tests {
    use super::super::execute::SearchResult;
    use super::super::{SearchCmd, SearchKind};
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

    // 3 modules in type_signatures: Accounts, Users, Repo
    crate::execute_test! {
        test_name: test_search_modules_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "MyApp".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.kind, "modules");
            assert_eq!(result.modules.len(), 3);
        },
    }

    // Functions with "user": get_user/1, get_user/2, list_users, create_user = 4
    crate::execute_test! {
        test_name: test_search_functions_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "user".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.kind, "functions");
            assert_eq!(result.functions.len(), 4);
        },
    }

    // Functions containing "get": get_user/1, get_user/2, get_by_email, Repo.get = 4
    crate::execute_test! {
        test_name: test_search_functions_specific,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "get".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 4);
        },
    }

    crate::execute_test! {
        test_name: test_search_functions_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^get_user$".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 2);
            assert!(result.functions.iter().all(|f| f.name == "get_user"));
        },
    }

    // Modules ending in Accounts or Users
    crate::execute_count_test! {
        test_name: test_search_modules_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "\\.(Accounts|Users)$".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        field: modules,
        expected: 2,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_search_modules_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "NonExistent".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        empty_field: modules,
    }

    crate::execute_no_match_test! {
        test_name: test_search_regex_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^xyz".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        empty_field: functions,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_search_modules_with_project_filter,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "App".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        collection: modules,
        condition: |m| m.project == "test_project",
    }

    crate::execute_limit_test! {
        test_name: test_search_with_limit,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "user".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 1,
            regex: false,
        },
        collection: functions,
        limit: 1,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: SearchCmd,
        cmd: SearchCmd {
            pattern: "test".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
    }
}
