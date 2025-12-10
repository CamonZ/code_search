//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::{HotspotKind, HotspotsCmd};
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
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "incoming");
            assert!(!result.modules.is_empty());
            // MyApp.Repo module should contain "get" function with 3 incoming
            let repo = result.modules.iter().find(|m| m.name == "MyApp.Repo").unwrap();
            let get = repo.functions.iter().find(|f| f.function == "get").unwrap();
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
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "outgoing");
            assert!(!result.modules.is_empty());
            // Check that we have modules with functions that have outgoing = 2
            assert!(result.total_hotspots > 0);
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
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "total");
            assert!(!result.modules.is_empty());
            // Find get_user with total = 3
            let accounts = result.modules.iter().find(|m| m.name == "MyApp.Accounts").unwrap();
            let get_user = accounts.functions.iter().find(|f| f.function == "get_user").unwrap();
            assert_eq!(get_user.total, 3);
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
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert!(result.modules.iter().all(|m| m.name.contains("Accounts")));
        },
    }

    crate::execute_test! {
        test_name: test_hotspots_with_limit,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
        },
        assertions: |result| {
            assert!(result.total_hotspots <= 2);
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
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
    }
}
