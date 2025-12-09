//! Execute tests for types command.

#[cfg(test)]
mod tests {
    use super::super::TypesCmd;
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
        test_name: test_types_finds_all_in_module,
        fixture: populated_db,
        cmd: TypesCmd {
            module: "MyApp.Accounts".to_string(),
            name: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.types.len(), 2);
            assert!(result.types.iter().all(|t| t.module == "MyApp.Accounts"));
        },
    }

    crate::execute_test! {
        test_name: test_types_filter_by_name,
        fixture: populated_db,
        cmd: TypesCmd {
            module: "MyApp.Accounts".to_string(),
            name: Some("user_id".to_string()),
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.types.len(), 1);
            assert_eq!(result.types[0].name, "user_id");
        },
    }

    crate::execute_test! {
        test_name: test_types_filter_by_kind,
        fixture: populated_db,
        cmd: TypesCmd {
            module: "MyApp".to_string(),
            name: None,
            kind: Some("opaque".to_string()),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.types.len(), 1);
            assert_eq!(result.types[0].kind, "opaque");
        },
    }

    crate::execute_test! {
        test_name: test_types_returns_definition,
        fixture: populated_db,
        cmd: TypesCmd {
            module: "MyApp.Accounts".to_string(),
            name: Some("user_id".to_string()),
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert!(!result.types[0].definition.is_empty());
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_types_no_match,
        fixture: populated_db,
        cmd: TypesCmd {
            module: "NonExistent".to_string(),
            name: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: types,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: TypesCmd,
        cmd: TypesCmd {
            module: "MyApp".to_string(),
            name: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
