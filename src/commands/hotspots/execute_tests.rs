//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::{HotspotKind, HotspotsCmd};
    use super::super::execute::HotspotsResult;
    use crate::commands::CommonArgs;
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

    // Test incoming hotspots shows proper formatting
    #[rstest]
    fn test_hotspots_incoming(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            assert_eq!(f.kind, "incoming");
            assert!(!f.entries.is_empty());
        } else {
            panic!("Expected Functions variant");
        }
    }

    #[rstest]
    fn test_hotspots_outgoing(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Outgoing,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            assert_eq!(f.kind, "outgoing");
            assert!(!f.entries.is_empty());
        } else {
            panic!("Expected Functions variant");
        }
    }

    #[rstest]
    fn test_hotspots_total(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Total,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            assert_eq!(f.kind, "total");
            assert!(!f.entries.is_empty());
        } else {
            panic!("Expected Functions variant");
        }
    }

    #[rstest]
    fn test_hotspots_ratio(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Ratio,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            assert_eq!(f.kind, "ratio");
            assert!(!f.entries.is_empty());
            // All entries should have a ratio value
            assert!(f.entries.iter().all(|e| e.ratio >= 0.0));
        } else {
            panic!("Expected Functions variant");
        }
    }

    // =========================================================================
    // Module-level tests (functions kind)
    // =========================================================================

    #[rstest]
    fn test_hotspots_functions_kind(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Functions,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Modules(m) = result {
            assert_eq!(m.kind, "functions");
            assert!(!m.entries.is_empty());
            // Check that entries have count set
            assert!(m.entries.iter().all(|e| e.count >= 0));
        } else {
            panic!("Expected Modules variant");
        }
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_hotspots_with_module_filter(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: Some("Accounts".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            // All entries should have Accounts in the module name
            assert!(f.entries.iter().all(|e| e.module.contains("Accounts")));
        } else {
            panic!("Expected Functions variant");
        }
    }

    #[rstest]
    fn test_hotspots_with_limit(populated_db: cozo::DbInstance) {
        let cmd = HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&populated_db).expect("Execute should succeed");

        if let HotspotsResult::Functions(f) = result {
            assert!(f.entries.len() <= 2);
        } else {
            panic!("Expected Functions variant");
        }
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: HotspotsCmd,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
    }
}
