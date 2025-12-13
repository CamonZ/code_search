//! Output formatting tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::HotspotEntry;
    use crate::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Hotspots (total)

No hotspots found.";

    const SINGLE_TABLE: &str = "\
Hotspots (total)

Found 1 hotspot(s) in 1 module(s):

MyApp.Accounts: (in: 3, out: 1, total: 4)
  get_user (in: 3, out: 1, total: 4)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<HotspotEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            kind_filter: Some("total".to_string()),
            function_pattern: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<HotspotEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            kind_filter: Some("total".to_string()),
            function_pattern: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![HotspotEntry {
                    function: "get_user".to_string(),
                    incoming: 3,
                    outgoing: 1,
                    total: 4,
                    ratio: 3.0,
                }],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> ModuleCollectionResult<HotspotEntry> {
        ModuleCollectionResult {
            module_pattern: "Service".to_string(),
            kind_filter: Some("outgoing".to_string()),
            function_pattern: None,
            name_filter: Some("Service".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Service".to_string(),
                file: String::new(),
                entries: vec![HotspotEntry {
                    function: "process".to_string(),
                    incoming: 0,
                    outgoing: 5,
                    total: 5,
                    ratio: 0.0,
                }],
                function_count: None,
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<HotspotEntry>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<HotspotEntry>,
        expected: SINGLE_TABLE,
    }

    #[rstest]
    fn test_to_table_filtered(filtered_result: ModuleCollectionResult<HotspotEntry>) {
        use crate::output::Outputable;
        let output = filtered_result.to_table();
        assert!(output.contains("(module: Service)"));
        assert!(output.contains("outgoing"));
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<HotspotEntry>,
        expected: crate::test_utils::load_output_fixture("hotspots", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<HotspotEntry>,
        expected: crate::test_utils::load_output_fixture("hotspots", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<HotspotEntry>,
        expected: crate::test_utils::load_output_fixture("hotspots", "empty.toon"),
        format: Toon,
    }
}
