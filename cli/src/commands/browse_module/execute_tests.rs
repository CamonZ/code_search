//! Execute tests for browse-module command.

#[cfg(test)]
mod tests {
    use super::super::{BrowseModuleCmd, DefinitionKind};
    use super::super::execute::Definition;
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Fixtures - call_graph has functions, specs, and types
    // =========================================================================

    crate::shared_fixture! {
        fixture_name: call_graph_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    crate::shared_fixture! {
        fixture_name: structs_db,
        fixture_type: structs,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests - Functions
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_finds_functions,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: Some(DefinitionKind::Functions),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(!result.definitions.is_empty());
            // All definitions should be functions
            for def in &result.definitions {
                assert!(matches!(def, Definition::Function { .. }));
            }
            // MyApp.Accounts has: get_user/1, get_user/2, list_users/0, validate_email/1
            assert_eq!(result.definitions.len(), 4);
        },
    }

    crate::execute_test! {
        test_name: test_browse_module_with_name_filter,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: Some(DefinitionKind::Functions),
            name: Some("get_user".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Only get_user/1 and get_user/2 should match
            assert_eq!(result.definitions.len(), 2);
            for def in &result.definitions {
                if let Definition::Function { name, .. } = def {
                    assert!(name.contains("get_user"));
                }
            }
        },
    }

    // =========================================================================
    // Core functionality tests - Specs
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_finds_specs,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: Some(DefinitionKind::Specs),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(!result.definitions.is_empty());
            // All definitions should be specs
            for def in &result.definitions {
                assert!(matches!(def, Definition::Spec { .. }));
            }
            // MyApp.Accounts has specs for: get_user/1, list_users/0
            assert_eq!(result.definitions.len(), 2);
        },
    }

    // =========================================================================
    // Core functionality tests - Types
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_finds_types,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: Some(DefinitionKind::Types),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert!(!result.definitions.is_empty());
            // All definitions should be types
            for def in &result.definitions {
                assert!(matches!(def, Definition::Type { .. }));
            }
            // MyApp.Accounts has types: user (type), user_id (opaque)
            assert_eq!(result.definitions.len(), 2);
        },
    }

    // =========================================================================
    // Core functionality tests - Structs
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_finds_structs,
        fixture: structs_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.User".to_string(),
            kind: Some(DefinitionKind::Structs),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            assert_eq!(result.definitions.len(), 1);
            if let Definition::Struct { name, fields, .. } = &result.definitions[0] {
                assert_eq!(name, "MyApp.User");
                // User has: id, name, email, admin, inserted_at
                assert_eq!(fields.len(), 5);
            } else {
                panic!("Expected struct definition");
            }
        },
    }

    // =========================================================================
    // Core functionality tests - All kinds (no filter)
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_all_kinds,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: None,  // No kind filter - get all
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        assertions: |result| {
            // Should have functions, specs, and types
            let has_functions = result.definitions.iter().any(|d| matches!(d, Definition::Function { .. }));
            let has_specs = result.definitions.iter().any(|d| matches!(d, Definition::Spec { .. }));
            let has_types = result.definitions.iter().any(|d| matches!(d, Definition::Type { .. }));

            assert!(has_functions, "Should have functions");
            assert!(has_specs, "Should have specs");
            assert!(has_types, "Should have types");

            // Functions: 4, Specs: 2, Types: 2 = 8 total
            assert_eq!(result.total_items, 8);
        },
    }

    // =========================================================================
    // Regex pattern tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_regex_pattern,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp\\..*".to_string(),
            kind: Some(DefinitionKind::Functions),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            // Should find functions from all MyApp.* modules
            assert!(!result.definitions.is_empty());
            // Total functions across all modules in fixture:
            // Controller: 3, Accounts: 4, Service: 3, Repo: 3, Notifier: 2 = 15
            assert_eq!(result.definitions.len(), 15);
        },
    }

    // =========================================================================
    // Sort order tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_sorted_by_module_then_line,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp\\..*".to_string(),
            kind: Some(DefinitionKind::Functions),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 100,
            },
        },
        assertions: |result| {
            // Verify sorted by module then by line
            let mut prev_module = String::new();
            let mut prev_line: i64 = 0;

            for def in &result.definitions {
                let (module, line) = match def {
                    Definition::Function { module, line, .. } => (module.clone(), *line),
                    _ => continue,
                };

                if module == prev_module {
                    assert!(line >= prev_line, "Within same module, lines should be ascending");
                } else if !prev_module.is_empty() {
                    assert!(module >= prev_module, "Modules should be in alphabetical order");
                }

                prev_module = module;
                prev_line = line;
            }
        },
    }

    // =========================================================================
    // Limit tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_browse_module_with_limit,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp\\..*".to_string(),
            kind: Some(DefinitionKind::Functions),
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: true,
                limit: 5,
            },
        },
        assertions: |result| {
            // Should respect limit
            assert_eq!(result.definitions.len(), 5);
            // total_items should reflect actual count before limit
            assert!(result.total_items >= 5);
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_browse_module_no_match,
        fixture: call_graph_db,
        cmd: BrowseModuleCmd {
            module_or_file: "NonExistent.Module".to_string(),
            kind: None,
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
        empty_field: definitions,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: BrowseModuleCmd,
        cmd: BrowseModuleCmd {
            module_or_file: "MyApp.Accounts".to_string(),
            kind: None,
            name: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 100,
            },
        },
    }
}
