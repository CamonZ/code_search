//! Execute tests for specs command.

#[cfg(test)]
mod tests {
    use super::super::SpecsCmd;
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
        test_name: test_specs_finds_all_in_module,
        fixture: populated_db,
        cmd: SpecsCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.specs.len(), 2);
            assert!(result.specs.iter().all(|s| s.module == "MyApp.Accounts"));
        },
    }

    crate::execute_test! {
        test_name: test_specs_filter_by_function,
        fixture: populated_db,
        cmd: SpecsCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.specs.len(), 1);
            assert_eq!(result.specs[0].name, "get_user");
            assert_eq!(result.specs[0].arity, 1);
        },
    }

    crate::execute_test! {
        test_name: test_specs_filter_by_kind,
        fixture: populated_db,
        cmd: SpecsCmd {
            module: "MyApp".to_string(),
            function: None,
            kind: Some("callback".to_string()),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.specs.len(), 1);
            assert_eq!(result.specs[0].kind, "callback");
            assert_eq!(result.specs[0].module, "MyApp.Repo");
        },
    }

    crate::execute_test! {
        test_name: test_specs_returns_full_signature,
        fixture: populated_db,
        cmd: SpecsCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.specs[0].inputs_string, "integer()");
            assert_eq!(result.specs[0].return_string, "{:ok, User.t()} | {:error, :not_found}");
            assert!(!result.specs[0].full.is_empty());
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_specs_no_match,
        fixture: populated_db,
        cmd: SpecsCmd {
            module: "NonExistent".to_string(),
            function: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: specs,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: SpecsCmd,
        cmd: SpecsCmd {
            module: "MyApp".to_string(),
            function: None,
            kind: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
