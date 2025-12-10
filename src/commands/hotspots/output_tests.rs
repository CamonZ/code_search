//! Output formatting tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::{HotspotEntry, HotspotModule, HotspotsResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Hotspots (incoming) in project 'test_project'

No hotspots found.";

    const SINGLE_TABLE: &str = "\
Hotspots (incoming) in project 'test_project'

Found 1 hotspot(s) in 1 module(s):

MyApp.Accounts:
  get_user (in: 3, out: 1, total: 4)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            total_hotspots: 0,
            modules: vec![],
        }
    }

    #[fixture]
    fn single_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            total_hotspots: 1,
            modules: vec![HotspotModule {
                name: "MyApp.Accounts".to_string(),
                functions: vec![HotspotEntry {
                    function: "get_user".to_string(),
                    incoming: 3,
                    outgoing: 1,
                    total: 4,
                }],
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "outgoing".to_string(),
            module_filter: Some("Service".to_string()),
            total_hotspots: 1,
            modules: vec![HotspotModule {
                name: "MyApp.Service".to_string(),
                functions: vec![HotspotEntry {
                    function: "process".to_string(),
                    incoming: 0,
                    outgoing: 5,
                    total: 5,
                }],
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: HotspotsResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: HotspotsResult,
        expected: SINGLE_TABLE,
    }

    #[rstest]
    fn test_to_table_filtered(filtered_result: HotspotsResult) {
        use crate::output::Outputable;
        let output = filtered_result.to_table();
        assert!(output.contains("(module filter: Service)"));
        assert!(output.contains("outgoing"));
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: HotspotsResult,
        expected: crate::test_utils::load_output_fixture("hotspots", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: HotspotsResult,
        expected: crate::test_utils::load_output_fixture("hotspots", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: HotspotsResult,
        expected: crate::test_utils::load_output_fixture("hotspots", "empty.toon"),
        format: Toon,
    }
}
