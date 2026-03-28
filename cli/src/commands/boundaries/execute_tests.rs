//! Execute tests for boundaries command.

#[cfg(test)]
mod tests {
    use super::super::BoundariesCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use crate::output::{OutputFormat, Outputable};
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_boundaries_basic(populated_db: Box<dyn db::backend::Database>) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind_filter, Some("boundary".to_string()));
        assert_eq!(result.module_pattern, "*");
        assert!(result.total_items > 0, "Should find boundary modules");
    }

    #[rstest]
    fn test_boundaries_result_structure(populated_db: Box<dyn db::backend::Database>) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for item in &result.items {
            // Each module should have exactly one entry
            assert_eq!(
                item.entries.len(),
                1,
                "Module {} should have exactly one entry",
                item.name
            );

            let entry = &item.entries[0];
            // All entries should meet the threshold criteria
            assert!(
                entry.incoming >= 1,
                "Module {} has {} incoming, expected >= 1",
                item.name,
                entry.incoming
            );
            assert!(
                entry.outgoing >= 1,
                "Module {} has {} outgoing, expected >= 1",
                item.name,
                entry.outgoing
            );
            assert!(
                entry.ratio >= 1.0,
                "Module {} has {:.2} ratio, expected >= 1.0",
                item.name,
                entry.ratio
            );
        }
    }

    // =========================================================================
    // Threshold boundary tests (exact values)
    // =========================================================================

    /// Tests the >= boundary for min_incoming by using the exact threshold value.
    /// In our fixture, MyApp.Notifier has incoming=2. Setting min_incoming=2
    /// should include it (>= is correct). If >= were replaced with <, it would be excluded.
    #[rstest]
    fn test_threshold_incoming_at_exact_boundary(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 2,
            min_ratio: 0.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Only MyApp.Notifier has incoming=2, others have incoming=1
        assert!(
            result.items.iter().any(|m| m.name == "MyApp.Notifier"),
            "MyApp.Notifier with incoming=2 should be included at min_incoming=2"
        );
        // Modules with incoming=1 should be excluded
        assert!(
            !result.items.iter().any(|m| m.name == "MyApp.Repo"),
            "MyApp.Repo with incoming=1 should be excluded at min_incoming=2"
        );
    }

    /// Tests the >= boundary for min_incoming just above the value.
    /// Setting min_incoming=3 should exclude MyApp.Notifier (incoming=2).
    #[rstest]
    fn test_threshold_incoming_above_boundary(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 3,
            min_ratio: 0.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(
            !result.items.iter().any(|m| m.name == "MyApp.Notifier"),
            "MyApp.Notifier with incoming=2 should be excluded at min_incoming=3"
        );
    }

    /// Tests the >= boundary for outgoing. The filter requires outgoing >= 1.
    /// MyApp.Controller has outgoing=3 but incoming=0, so it should be excluded
    /// by the incoming threshold (min_incoming=1), but included with min_incoming=0.
    #[rstest]
    fn test_threshold_outgoing_at_boundary(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        // With min_incoming=0, min_ratio=0.0, modules with outgoing >= 1 should be included
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 0,
            min_ratio: 0.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Controller has outgoing=3, so it passes the outgoing >= 1 check
        assert!(
            result.items.iter().any(|m| m.name == "MyApp.Controller"),
            "MyApp.Controller with outgoing=3 should be included when min_incoming=0"
        );
    }

    /// Tests the >= boundary for min_ratio at the exact threshold.
    /// With min_ratio=1.0, modules with ratio=1.0 should be included.
    /// If >= were replaced with <, they would be excluded.
    #[rstest]
    fn test_threshold_ratio_at_exact_boundary(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Many modules have ratio=1.0, they should all be included
        assert!(
            result.total_items >= 7,
            "Should include modules with ratio=1.0 when min_ratio=1.0, got {}",
            result.total_items
        );
    }

    /// Tests that ratio just above the data excludes modules.
    /// All modules in the fixture have ratio <= 1.0. Setting min_ratio=1.1
    /// should exclude all of them.
    #[rstest]
    fn test_threshold_ratio_above_boundary(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.1,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 0,
            "No modules should have ratio >= 1.1 in this fixture"
        );
    }

    // =========================================================================
    // Compound filter tests (AND logic)
    // =========================================================================

    /// Tests that only incoming being true is insufficient -- outgoing must also pass.
    /// MyApp.Controller has incoming=0, outgoing=3. With min_incoming=0, it would pass
    /// the incoming check. But Service has incoming=1, outgoing=3, ratio=0.33.
    /// With min_ratio=0.5, Service should be excluded even though incoming and outgoing pass.
    #[rstest]
    fn test_compound_filter_ratio_fails_but_others_pass(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 0.5,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // MyApp.Service has ratio=0.33 which is below 0.5 threshold
        assert!(
            !result.items.iter().any(|m| m.name == "MyApp.Service"),
            "MyApp.Service with ratio=0.33 should be excluded when min_ratio=0.5"
        );

        // But modules with ratio=1.0 should be included
        assert!(
            result.items.iter().any(|m| m.name == "MyApp.Accounts"),
            "MyApp.Accounts with ratio=1.0 should be included when min_ratio=0.5"
        );
    }

    /// Tests that high incoming alone is insufficient if ratio doesn't meet threshold.
    /// MyApp.Notifier has incoming=2 and ratio=1.0. With min_ratio=2.0,
    /// it should be excluded despite having the highest incoming count.
    #[rstest]
    fn test_compound_filter_incoming_passes_ratio_fails(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 2,
            min_ratio: 2.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(
            result.total_items, 0,
            "No modules should pass both min_incoming=2 and min_ratio=2.0"
        );
    }

    /// Tests that all three conditions must be true simultaneously.
    /// This ensures && wasn't replaced with ||.
    #[rstest]
    fn test_compound_filter_all_must_pass(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        // Use thresholds that no single module satisfies all of:
        // min_incoming=2 (only Notifier passes) AND min_ratio=2.0 (no module passes)
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 2,
            min_ratio: 2.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // With && logic: nothing passes (Notifier has incoming=2 but ratio=1.0)
        // With || logic: Notifier would pass (incoming=2 >= 2 is true, making || true)
        assert_eq!(
            result.total_items, 0,
            "With && logic, no module should pass both incoming>=2 AND ratio>=2.0. \
             If this fails, && may have been replaced with ||"
        );
    }

    // =========================================================================
    // Filter and limit tests
    // =========================================================================

    #[rstest]
    fn test_boundaries_with_module_filter(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: Some("MyApp.Notifier".to_string()),
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.module_pattern, "MyApp.Notifier");
        // All results should match the filter
        for item in &result.items {
            assert!(
                item.name.contains("Notifier"),
                "Module {} doesn't match filter 'MyApp.Notifier'",
                item.name
            );
        }
    }

    #[rstest]
    fn test_boundaries_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = BoundariesCmd {
            module: Some("NonExistentModule".to_string()),
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
    }

    #[rstest]
    fn test_boundaries_high_threshold_filters_all(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 999999,
            min_ratio: 999.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_items, 0);
        assert!(result.items.is_empty());
    }

    // =========================================================================
    // run() integration test (mod.rs:44 mutant)
    // =========================================================================

    /// Tests that run() produces non-empty, correct output.
    /// This kills the run() -> Ok(String::new()) and Ok("xyzzy") mutants.
    #[rstest]
    fn test_run_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Must not be empty (kills -> Ok(String::new()))
        assert!(!output.is_empty(), "run() should produce non-empty output");
        // Must contain expected content (kills -> Ok("xyzzy"))
        assert!(
            output.contains("Boundary Modules"),
            "Output should contain 'Boundary Modules', got: {}",
            output
        );
        // Verify it contains actual data
        assert!(
            output.contains("boundary module(s)"),
            "Output should contain results summary"
        );
    }

    /// Tests run() with empty results to ensure even empty output is correct.
    #[rstest]
    fn test_run_empty_produces_correct_output(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 999999,
            min_ratio: 999.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        assert!(
            output.contains("Boundary Modules"),
            "Even empty output should have header"
        );
        assert!(
            output.contains("No boundary modules found."),
            "Empty output should show empty message"
        );
    }

    /// Tests run() with JSON format to kill mutants via format path.
    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = BoundariesCmd {
            module: None,
            min_incoming: 1,
            min_ratio: 1.0,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        // Must be valid JSON (kills String::new() and "xyzzy")
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");
        assert_eq!(parsed["kind_filter"], "boundary");
        assert!(parsed["total_items"].as_u64().unwrap() > 0);
    }
}
