//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::HotspotsResult;
    use super::super::{HotspotKind, HotspotsCmd};
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

    // Highest incoming: Repo.get with 3 (from get_user/1, get_user/2, do_fetch)
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
            assert!(!result.hotspots.is_empty());
            assert_eq!(result.hotspots[0].function, "get");
            assert_eq!(result.hotspots[0].incoming, 3);
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
            assert!(!result.hotspots.is_empty());
            // Both get_user and process have 2 outgoing; Accounts.get_user comes first alphabetically
            assert_eq!(result.hotspots[0].outgoing, 2);
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
            assert!(!result.hotspots.is_empty());
            assert_eq!(result.hotspots[0].function, "get_user");
            assert_eq!(result.hotspots[0].total, 3);
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_hotspots_with_module_filter,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: Some("Accounts".to_string()),
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        collection: hotspots,
        condition: |h| h.module.contains("Accounts"),
    }

    crate::execute_limit_test! {
        test_name: test_hotspots_with_limit,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
        },
        collection: hotspots,
        limit: 2,
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
