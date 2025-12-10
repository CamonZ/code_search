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
            assert_eq!(result.total_specs, 2);
            assert_eq!(result.modules.len(), 1);
            assert_eq!(result.modules[0].name, "MyApp.Accounts");
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
            assert_eq!(result.total_specs, 1);
            let spec = &result.modules[0].specs[0];
            assert_eq!(spec.name, "get_user");
            assert_eq!(spec.arity, 1);
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
            assert_eq!(result.total_specs, 1);
            let spec = &result.modules[0].specs[0];
            assert_eq!(spec.kind, "callback");
            assert_eq!(result.modules[0].name, "MyApp.Repo");
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
            let spec = &result.modules[0].specs[0];
            assert_eq!(spec.inputs, "integer()");
            assert_eq!(spec.returns, "{:ok, User.t()} | {:error, :not_found}");
            assert!(!spec.full.is_empty());
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
        empty_field: modules,
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
