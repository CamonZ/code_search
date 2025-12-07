//! Output formatting tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::{Hotspot, HotspotsResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Hotspots (incoming) in project 'test_project'

No hotspots found.";

    const SINGLE_TABLE: &str = "\
Hotspots (incoming) in project 'test_project'

FUNCTION                                                 IN      OUT    TOTAL
------------------------------------------------------------------------------
MyApp.Accounts.get_user                                   3        1        4";

    const SINGLE_JSON: &str = r#"{
  "project": "test_project",
  "kind": "incoming",
  "module_filter": null,
  "hotspots": [
    {
      "module": "MyApp.Accounts",
      "function": "get_user",
      "incoming": 3,
      "outgoing": 1,
      "total": 4
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
hotspots[1]{function,incoming,module,outgoing,total}:
  get_user,3,MyApp.Accounts,1,4
kind: incoming
module_filter: null
project: test_project";

    const EMPTY_TOON: &str = "\
hotspots[0]:
kind: incoming
module_filter: null
project: test_project";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            hotspots: vec![],
        }
    }

    #[fixture]
    fn single_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            hotspots: vec![Hotspot {
                module: "MyApp.Accounts".to_string(),
                function: "get_user".to_string(),
                incoming: 3,
                outgoing: 1,
                total: 4,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "outgoing".to_string(),
            module_filter: Some("Service".to_string()),
            hotspots: vec![Hotspot {
                module: "MyApp.Service".to_string(),
                function: "process".to_string(),
                incoming: 0,
                outgoing: 5,
                total: 5,
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
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: HotspotsResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: HotspotsResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
