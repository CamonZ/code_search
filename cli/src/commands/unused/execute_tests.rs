//! Execute tests for unused command.

#[cfg(test)]
mod tests {
    use super::super::UnusedCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::collections::HashSet;

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // The SurrealDB complex fixture has 16 unused functions:
    // - 3 private: validate_email, debug, transform_data
    // - 13 public: __struct__, __generated__ x2, format_name, format_display,
    //              fetch, create, index, show, subscribe, increment, validate x2
    #[rstest]
    fn test_unused_finds_uncalled_functions(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 16, "Should find 16 unused functions");

        let all_funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();

        assert!(all_funcs.contains("validate_email"));
        assert!(all_funcs.contains("transform_data"));
        assert!(all_funcs.contains("index"));
        assert!(all_funcs.contains("show"));
    }

    // Accounts has 4 unused: __generated__, __struct__, format_name, validate_email
    #[rstest]
    fn test_unused_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("MyApp.Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 4,
            "Accounts should have 4 unused functions"
        );

        // Verify all results are from MyApp.Accounts
        for module_group in &result.items {
            assert_eq!(module_group.name, "MyApp.Accounts");
        }

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("__generated__"));
        assert!(funcs.contains("__struct__"));
        assert!(funcs.contains("format_name"));
        assert!(funcs.contains("validate_email"));
    }

    // Controller has 5 unused: __generated__, create, format_display, index, show
    #[rstest]
    fn test_unused_with_regex_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("^MyApp\\.Controller$".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: true,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 5,
            "Controller should have 5 unused functions"
        );

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("__generated__"));
        assert!(funcs.contains("create"));
        assert!(funcs.contains("format_display"));
        assert!(funcs.contains("index"));
        assert!(funcs.contains("show"));
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    #[rstest]
    fn test_unused_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("NonExistent".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(result.items.is_empty(), "Expected no results for non-existent module");
        assert_eq!(result.total_items, 0);
    }

    #[rstest]
    fn test_unused_exact_no_partial(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("Accounts".to_string()), // Won't match "MyApp.Accounts"
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(
            result.items.is_empty(),
            "Partial match 'Accounts' should not match 'MyApp.Accounts'"
        );
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_unused_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 3,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");
        assert_eq!(result.total_items, 3, "Limit should restrict to 3 results");
    }

    // 3 private unused: validate_email, debug, transform_data
    #[rstest]
    fn test_unused_private_only(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 3,
            "Should find 3 private unused functions"
        );

        let funcs: HashSet<&str> = result
            .items
            .iter()
            .flat_map(|m| m.entries.iter().map(|f| f.name.as_str()))
            .collect();
        assert!(funcs.contains("validate_email"));
        assert!(funcs.contains("debug"));
        assert!(funcs.contains("transform_data"));

        // All should be private (defp or defmacrop)
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    func.kind == "defp" || func.kind == "defmacrop",
                    "Expected private function, got {} for {}",
                    func.kind,
                    func.name
                );
            }
        }
    }

    // 13 public unused
    #[rstest]
    fn test_unused_public_only(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 13,
            "Should find 13 public unused functions"
        );

        // All should be public (def or defmacro)
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    func.kind == "def" || func.kind == "defmacro",
                    "Expected public function, got {} for {}",
                    func.kind,
                    func.name
                );
            }
        }
    }

    // Excluding generated should reduce from 16 to 13
    #[rstest]
    fn test_unused_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: true,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 13,
            "Excluding generated should leave 13 functions (16 - 3 generated)"
        );

        // Verify no generated functions
        for module in &result.items {
            for func in &module.entries {
                assert!(
                    !func.name.starts_with("__"),
                    "Should not contain generated function: {}",
                    func.name
                );
            }
        }
    }

    // Combined: public + exclude_generated = 10 (13 public - 3 generated)
    #[rstest]
    fn test_unused_public_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: true,
            exclude_generated: true,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 10,
            "Public + exclude_generated should leave 10 functions"
        );
    }

    // Notifier has no unused functions (all are called)
    #[rstest]
    fn test_unused_notifier_empty(populated_db: Box<dyn db::backend::Database>) {
        let cmd = UnusedCmd {
            module: Some("MyApp.Notifier".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(
            result.items.is_empty(),
            "Notifier should have no unused functions"
        );
        assert_eq!(result.total_items, 0);
    }

    // =========================================================================
    // CommandRunner::run() integration tests
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(
            output.contains("Unused functions"),
            "Table output should contain header, got:\n{}",
            output
        );
        assert!(
            output.contains("Found 16 unused function(s) in"),
            "Table should contain summary with 16 functions, got:\n{}",
            output
        );
    }

    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = UnusedCmd {
            module: Some("NonExistent".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run should succeed");

        assert!(
            output.contains("Unused functions"),
            "Header should be present"
        );
        assert!(
            output.contains("No unused functions found."),
            "Empty result should show empty message, got:\n{}",
            output
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = UnusedCmd {
            module: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["module_pattern"], "*");
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["total_items"], 16);
    }
}
