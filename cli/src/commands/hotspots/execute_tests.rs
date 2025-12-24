//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::HotspotsCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use db::queries::hotspots::HotspotKind;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_hotspots_incoming(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "incoming");
        assert!(!result.entries.is_empty());
    }

    #[rstest]
    fn test_hotspots_outgoing(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Outgoing,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "outgoing");
        assert!(!result.entries.is_empty());
    }

    #[rstest]
    fn test_hotspots_total(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Total,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "total");
        assert!(!result.entries.is_empty());
    }

    #[rstest]
    fn test_hotspots_ratio(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Ratio,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "ratio");
        assert!(!result.entries.is_empty());
        // All entries should have a ratio value
        assert!(result.entries.iter().all(|e| e.ratio >= 0.0));
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_hotspots_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: Some("Accounts".to_string()),
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All entries should have Accounts in the module name
        assert!(result.entries.iter().all(|e| e.module.contains("Accounts")));
    }

    #[rstest]
    fn test_hotspots_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.entries.len() <= 2);
    }

    #[rstest]
    fn test_hotspots_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: true,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // With exclude_generated, generated functions should be filtered out
        // Result may or may not be empty depending on test data
        assert_eq!(result.kind, "incoming");
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: HotspotsCmd,
        cmd: HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
    }
}
