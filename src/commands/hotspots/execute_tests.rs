//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::{HotspotKind, HotspotsCmd};
    use crate::commands::CommonArgs;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    // Highest incoming: MyApp.Repo.get with 3 (from get_user/1, get_user/2, do_fetch)
    crate::execute_test! {
        test_name: test_hotspots_incoming,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind_filter, Some("incoming".to_string()));
            assert!(!result.items.is_empty());
            // MyApp.Repo module should contain "get" function with 3 incoming
            let repo = result.items.iter().find(|m| m.name == "MyApp.Repo").unwrap();
            let get = repo.entries.iter().find(|f| f.function == "get").unwrap();
            assert_eq!(get.incoming, 3);
        },
    }

    // Highest outgoing: get_user or process with 2 each (sorted alphabetically)
    crate::execute_test! {
        test_name: test_hotspots_outgoing,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Outgoing,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind_filter, Some("outgoing".to_string()));
            assert!(!result.items.is_empty());
            // Check that we have modules with functions that have outgoing = 2
            assert!(result.total_items > 0);
        },
    }

    // Highest total: get_user (1 incoming + 2 outgoing = 3), get also has 3
    // Accounts.get_user sorts before Repo.get alphabetically
    crate::execute_test! {
        test_name: test_hotspots_total,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Total,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind_filter, Some("total".to_string()));
            assert!(!result.items.is_empty());
            // Find get_user with total = 3
            let accounts = result.items.iter().find(|m| m.name == "MyApp.Accounts").unwrap();
            let get_user = accounts.entries.iter().find(|f| f.function == "get_user").unwrap();
            assert_eq!(get_user.total, 3);
        },
    }

    // Ratio: boundary modules with high incoming/outgoing ratio
    // Functions with 0 outgoing get ratio = incoming * 1000
    crate::execute_test! {
        test_name: test_hotspots_ratio,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Ratio,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
        assertions: |result| {
            assert_eq!(result.kind_filter, Some("ratio".to_string()));
            assert!(!result.items.is_empty());
            // Check that ratio values are calculated and populated
            let all_entries: Vec<_> = result.items.iter()
                .flat_map(|m| &m.entries)
                .collect();
            assert!(all_entries.len() > 0);
            // All entries should have a ratio value
            assert!(all_entries.iter().all(|e| e.ratio >= 0.0));
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_hotspots_with_module_filter,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: Some("Accounts".to_string()),
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 20,
            },
        },
        assertions: |result| {
            assert!(result.items.iter().all(|m| m.name.contains("Accounts")));
        },
    }

    crate::execute_test! {
        test_name: test_hotspots_with_limit,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            common: CommonArgs {
                project: "test_project".to_string(),
                regex: false,
                limit: 2,
            },
        },
        assertions: |result| {
            assert!(result.total_items <= 2);
        },
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
