//! Output formatting tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::{FunctionHotspotEntry, HotspotsResult};
    use crate::output::{OutputFormat, Outputable};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Hotspots (incoming)

No hotspots found.";

    const SINGLE_TABLE: &str = "\
Hotspots (total)

Found 1 function:

MyApp.Accounts.get_user  3 in  1 out  4 total    0.25 ratio";

    const MULTIPLE_TABLE: &str = "\
Hotspots (incoming)

Found 2 functions:

MyApp.Accounts.get_user  10 in  2 out  12 total    0.17 ratio
MyApp.Users.create        5 in  3 out   8 total    0.38 ratio";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> HotspotsResult {
        HotspotsResult {
            kind: "incoming".to_string(),
            total_items: 0,
            entries: vec![],
        }
    }

    #[fixture]
    fn single_result() -> HotspotsResult {
        HotspotsResult {
            kind: "total".to_string(),
            total_items: 1,
            entries: vec![FunctionHotspotEntry {
                module: "MyApp.Accounts".to_string(),
                function: "get_user".to_string(),
                incoming: 3,
                outgoing: 1,
                total: 4,
                ratio: 0.25,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> HotspotsResult {
        HotspotsResult {
            kind: "incoming".to_string(),
            total_items: 2,
            entries: vec![
                FunctionHotspotEntry {
                    module: "MyApp.Accounts".to_string(),
                    function: "get_user".to_string(),
                    incoming: 10,
                    outgoing: 2,
                    total: 12,
                    ratio: 0.17,
                },
                FunctionHotspotEntry {
                    module: "MyApp.Users".to_string(),
                    function: "create".to_string(),
                    incoming: 5,
                    outgoing: 3,
                    total: 8,
                    ratio: 0.38,
                },
            ],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    #[rstest]
    fn test_to_table_empty(empty_result: HotspotsResult) {
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_to_table_single(single_result: HotspotsResult) {
        let output = single_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: HotspotsResult) {
        let output = multiple_result.to_table();
        assert_eq!(output, MULTIPLE_TABLE);
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_format_json(single_result: HotspotsResult) {
        let output = single_result.format(OutputFormat::Json);
        assert!(output.contains("\"kind\": \"total\""));
        assert!(output.contains("\"total_items\": 1"));
        assert!(output.contains("\"entries\""));
        assert!(output.contains("\"module\": \"MyApp.Accounts\""));
        assert!(output.contains("\"function\": \"get_user\""));
        assert!(output.contains("\"incoming\": 3"));
        assert!(output.contains("\"outgoing\": 1"));
        assert!(output.contains("\"total\": 4"));
    }

    #[rstest]
    fn test_format_json_empty(empty_result: HotspotsResult) {
        let output = empty_result.format(OutputFormat::Json);
        assert!(output.contains("\"kind\": \"incoming\""));
        assert!(output.contains("\"total_items\": 0"));
        assert!(output.contains("\"entries\": []"));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_format_toon(single_result: HotspotsResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("kind"));
        assert!(output.contains("total_items"));
        assert!(output.contains("entries"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: HotspotsResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("kind"));
        assert!(output.contains("entries"));
    }
}
