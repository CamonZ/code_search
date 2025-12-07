//! Execute tests for struct command.

#[cfg(test)]
mod tests {
    use super::super::execute::StructResult;
    use super::super::StructCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: structs,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // User struct has 5 fields: id, name, email, admin, inserted_at
    crate::execute_test! {
        test_name: test_struct_exact_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.structs.len(), 1);
            assert_eq!(result.structs[0].module, "MyApp.User");
            assert_eq!(result.structs[0].fields.len(), 5);
        },
    }

    crate::execute_test! {
        test_name: test_struct_fields_content,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            let user_struct = &result.structs[0];
            let email_field = user_struct.fields.iter().find(|f| f.name == "email").unwrap();
            assert!(email_field.required);
            assert_eq!(email_field.inferred_type, "String.t()");
        },
    }

    // 3 structs: User, Post, Comment
    crate::execute_count_test! {
        test_name: test_struct_regex_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: structs,
        expected: 3,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_struct_no_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "NonExistent".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: structs,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_with_project_filter,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.structs.len(), 1);
            assert_eq!(result.structs[0].project, "test_project");
        },
    }

    crate::execute_test! {
        test_name: test_struct_with_limit,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 3,
        },
        assertions: |result| {
            let total_fields: usize = result.structs.iter().map(|s| s.fields.len()).sum();
            assert!(total_fields <= 3);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: StructCmd,
        cmd: StructCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
