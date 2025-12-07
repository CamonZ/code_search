//! Execute tests for file command.

#[cfg(test)]
mod tests {
    use super::super::execute::FileResult;
    use super::super::FileCmd;
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

    // accounts.ex has 4 functions: get_user/1, get_user/2, list_users, validate_email
    crate::execute_test! {
        test_name: test_file_finds_functions,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/my_app/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.files.len(), 1);
            assert_eq!(result.files[0].file, "lib/my_app/accounts.ex");
            assert_eq!(result.files[0].functions.len(), 4);
        },
    }

    crate::execute_test! {
        test_name: test_file_substring_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "accounts".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.files.len(), 1);
            assert_eq!(result.files[0].file, "lib/my_app/accounts.ex");
        },
    }

    // All 5 files are in lib/my_app/*.ex format
    crate::execute_count_test! {
        test_name: test_file_regex_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "^lib/my_app/[^/]+\\.ex$".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: files,
        expected: 5,
    }

    // All 5 files are under lib/
    crate::execute_count_test! {
        test_name: test_file_multiple_files,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        field: files,
        expected: 5,
    }

    // Functions sorted by line: get_user/1 (10), get_user/2 (17), list_users (24), validate_email (30)
    crate::execute_test! {
        test_name: test_file_sorted_by_line,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/my_app/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            let funcs = &result.files[0].functions;
            assert_eq!(funcs[0].name, "get_user");
            assert_eq!(funcs[0].start_line, 10);
            assert_eq!(funcs[1].name, "get_user");
            assert_eq!(funcs[1].start_line, 17);
            assert_eq!(funcs[2].name, "list_users");
            assert_eq!(funcs[3].name, "validate_email");
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_file_no_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "nonexistent.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: files,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_file_with_limit,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
        },
        assertions: |result| {
            let total_funcs: usize = result.files.iter().map(|f| f.functions.len()).sum();
            assert!(total_funcs <= 2);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: FileCmd,
        cmd: FileCmd {
            file: "lib/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
