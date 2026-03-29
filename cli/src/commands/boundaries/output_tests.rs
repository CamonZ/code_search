//! Output formatting tests for boundaries command.

#[cfg(test)]
mod tests {
    use super::super::execute::BoundaryEntry;
    use crate::output::{OutputFormat, Outputable, TableFormatter};
    use db::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Boundary Modules

No boundary modules found.";

    const EMPTY_TABLE_WITH_FILTER: &str = "\
Boundary Modules (module: MyApp.Web)

No boundary modules found.";

    const SINGLE_TABLE: &str = "\
Boundary Modules

Found 1 boundary module(s) in 1 module(s):

MyApp.Notifier: (in: 5, out: 2, ratio: 2.5)
  (in: 5, out: 2, ratio: 2.5)";

    const MULTIPLE_TABLE: &str = "\
Boundary Modules

Found 2 boundary module(s) in 2 module(s):

MyApp.Accounts: (in: 10, out: 3, ratio: 3.3)
  (in: 10, out: 3, ratio: 3.3)

MyApp.Repo: (in: 4, out: 1, ratio: 4.0)
  (in: 4, out: 1, ratio: 4.0)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<BoundaryEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn empty_result_with_filter() -> ModuleCollectionResult<BoundaryEntry> {
        ModuleCollectionResult {
            module_pattern: "MyApp.Web".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<BoundaryEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Notifier".to_string(),
                file: String::new(),
                entries: vec![BoundaryEntry {
                    incoming: 5,
                    outgoing: 2,
                    ratio: 2.5,
                }],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleCollectionResult<BoundaryEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 2,
            items: vec![
                ModuleGroup {
                    name: "MyApp.Accounts".to_string(),
                    file: String::new(),
                    entries: vec![BoundaryEntry {
                        incoming: 10,
                        outgoing: 3,
                        ratio: 3.3,
                    }],
                    function_count: None,
                },
                ModuleGroup {
                    name: "MyApp.Repo".to_string(),
                    file: String::new(),
                    entries: vec![BoundaryEntry {
                        incoming: 4,
                        outgoing: 1,
                        ratio: 4.0,
                    }],
                    function_count: None,
                },
            ],
        }
    }

    #[fixture]
    fn infinite_ratio_result() -> ModuleCollectionResult<BoundaryEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Leaf".to_string(),
                file: String::new(),
                entries: vec![BoundaryEntry {
                    incoming: 5,
                    outgoing: 0,
                    ratio: f64::INFINITY,
                }],
                function_count: None,
            }],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    #[rstest]
    fn test_to_table_empty(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_to_table_empty_with_filter(
        empty_result_with_filter: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let output = empty_result_with_filter.to_table();
        assert_eq!(output, EMPTY_TABLE_WITH_FILTER);
    }

    #[rstest]
    fn test_to_table_single(single_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = single_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = multiple_result.to_table();
        assert_eq!(output, MULTIPLE_TABLE);
    }

    // =========================================================================
    // format_header tests
    // =========================================================================

    #[rstest]
    fn test_format_header_no_filter(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let header = empty_result.format_header();
        assert_eq!(header, "Boundary Modules");
    }

    #[rstest]
    fn test_format_header_with_filter(
        empty_result_with_filter: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let header = empty_result_with_filter.format_header();
        assert_eq!(header, "Boundary Modules (module: MyApp.Web)");
    }

    /// Kills the != replaced with == mutant on line 11.
    /// When module_pattern is "*", the filter_info branch should produce an empty string.
    /// If != were replaced with ==, the wildcard pattern would incorrectly show a filter suffix.
    #[test]
    fn test_format_header_wildcard_has_no_filter_suffix() {
        let result = ModuleCollectionResult::<BoundaryEntry> {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 0,
            items: vec![],
        };
        let header = result.format_header();
        assert!(!header.contains("(module:"), "Wildcard pattern should not show filter");
        assert_eq!(header, "Boundary Modules");
    }

    /// Ensures that a non-wildcard module_pattern includes the filter suffix.
    #[test]
    fn test_format_header_non_wildcard_shows_filter() {
        let result = ModuleCollectionResult::<BoundaryEntry> {
            module_pattern: "MyApp.Web".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 0,
            items: vec![],
        };
        let header = result.format_header();
        assert!(header.contains("(module: MyApp.Web)"));
    }

    // =========================================================================
    // format_empty_message tests
    // =========================================================================

    #[rstest]
    fn test_format_empty_message(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let msg = empty_result.format_empty_message();
        assert_eq!(msg, "No boundary modules found.");
    }

    // =========================================================================
    // format_summary tests
    // =========================================================================

    #[rstest]
    fn test_format_summary(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let summary = empty_result.format_summary(3, 2);
        assert_eq!(summary, "Found 3 boundary module(s) in 2 module(s):");
    }

    #[rstest]
    fn test_format_summary_single(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let summary = empty_result.format_summary(1, 1);
        assert_eq!(summary, "Found 1 boundary module(s) in 1 module(s):");
    }

    // =========================================================================
    // format_module_header tests
    // =========================================================================

    #[rstest]
    fn test_format_module_header(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let header = empty_result.format_module_header("MyApp.Accounts", "");
        assert_eq!(header, "MyApp.Accounts:");
    }

    // =========================================================================
    // format_module_header_with_entries tests
    // =========================================================================

    #[rstest]
    fn test_format_module_header_with_entries(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entries = vec![BoundaryEntry {
            incoming: 10,
            outgoing: 3,
            ratio: 3.3,
        }];
        let header =
            empty_result.format_module_header_with_entries("MyApp.Accounts", "", &entries);
        assert_eq!(
            header,
            "MyApp.Accounts: (in: 10, out: 3, ratio: 3.3)"
        );
    }

    /// When entries is empty, format_module_header_with_entries falls back to simple header.
    #[rstest]
    fn test_format_module_header_with_entries_empty(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entries: Vec<BoundaryEntry> = vec![];
        let header =
            empty_result.format_module_header_with_entries("MyApp.Accounts", "", &entries);
        assert_eq!(header, "MyApp.Accounts:");
    }

    /// Kills the == replaced with != mutant on line 45 (outgoing == 0 check).
    /// When outgoing is 0, ratio should display as infinity symbol.
    #[rstest]
    fn test_format_module_header_with_entries_infinite_ratio(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entries = vec![BoundaryEntry {
            incoming: 5,
            outgoing: 0,
            ratio: f64::INFINITY,
        }];
        let header =
            empty_result.format_module_header_with_entries("MyApp.Leaf", "", &entries);
        assert!(
            header.contains("\u{221e}"),
            "Should contain infinity symbol when outgoing is 0, got: {}",
            header
        );
        assert_eq!(header, "MyApp.Leaf: (in: 5, out: 0, ratio: \u{221e})");
    }

    /// Kills the == replaced with != mutant on line 45.
    /// When outgoing is non-zero, ratio should display as a decimal, NOT infinity.
    #[rstest]
    fn test_format_module_header_with_entries_non_zero_outgoing(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entries = vec![BoundaryEntry {
            incoming: 6,
            outgoing: 2,
            ratio: 3.0,
        }];
        let header =
            empty_result.format_module_header_with_entries("MyApp.Service", "", &entries);
        assert!(
            !header.contains("\u{221e}"),
            "Should NOT contain infinity when outgoing > 0"
        );
        assert_eq!(header, "MyApp.Service: (in: 6, out: 2, ratio: 3.0)");
    }

    // =========================================================================
    // format_entry tests
    // =========================================================================

    #[rstest]
    fn test_format_entry(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let entry = BoundaryEntry {
            incoming: 8,
            outgoing: 2,
            ratio: 4.0,
        };
        let output = empty_result.format_entry(&entry, "MyApp.Service", "");
        assert_eq!(output, "(in: 8, out: 2, ratio: 4.0)");
    }

    /// Kills the == replaced with != mutant on line 60 (outgoing == 0 check in format_entry).
    #[rstest]
    fn test_format_entry_infinite_ratio(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entry = BoundaryEntry {
            incoming: 3,
            outgoing: 0,
            ratio: f64::INFINITY,
        };
        let output = empty_result.format_entry(&entry, "MyApp.Leaf", "");
        assert!(
            output.contains("\u{221e}"),
            "Should contain infinity symbol when outgoing is 0"
        );
        assert_eq!(output, "(in: 3, out: 0, ratio: \u{221e})");
    }

    /// Kills the == replaced with != mutant on line 60.
    /// When outgoing is non-zero, the entry should NOT contain infinity.
    #[rstest]
    fn test_format_entry_non_zero_outgoing(
        empty_result: ModuleCollectionResult<BoundaryEntry>,
    ) {
        let entry = BoundaryEntry {
            incoming: 6,
            outgoing: 3,
            ratio: 2.0,
        };
        let output = empty_result.format_entry(&entry, "MyApp.Test", "");
        assert!(
            !output.contains("\u{221e}"),
            "Should NOT contain infinity when outgoing > 0"
        );
        assert_eq!(output, "(in: 6, out: 3, ratio: 2.0)");
    }

    // =========================================================================
    // blank_before_module / blank_after_summary tests
    // =========================================================================

    #[rstest]
    fn test_blank_before_module(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        assert!(
            empty_result.blank_before_module(),
            "blank_before_module should return true for boundaries"
        );
    }

    #[rstest]
    fn test_blank_after_summary(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        assert!(
            !empty_result.blank_after_summary(),
            "blank_after_summary should return false for boundaries"
        );
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_format_json(single_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Should produce valid JSON");
        assert_eq!(parsed["module_pattern"], "*");
        assert_eq!(parsed["kind_filter"], "boundary");
        assert_eq!(parsed["total_items"], 1);
        assert_eq!(parsed["items"][0]["name"], "MyApp.Notifier");
        assert_eq!(parsed["items"][0]["entries"][0]["incoming"], 5);
        assert_eq!(parsed["items"][0]["entries"][0]["outgoing"], 2);
        assert_eq!(parsed["items"][0]["entries"][0]["ratio"], 2.5);
    }

    #[rstest]
    fn test_format_json_empty(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = empty_result.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Should produce valid JSON");
        assert_eq!(parsed["total_items"], 0);
        assert_eq!(parsed["items"], serde_json::json!([]));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_format_toon(single_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("MyApp.Notifier"));
        assert!(output.contains("boundary"));
        assert!(output.contains("total_items"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: ModuleCollectionResult<BoundaryEntry>) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("boundary"));
        assert!(output.contains("total_items"));
    }

    // =========================================================================
    // Full table rendering tests (tests run() -> format pipeline)
    // =========================================================================

    /// Tests that the table output for an infinite ratio entry renders correctly.
    /// This exercises format_module_header_with_entries and format_entry together.
    #[test]
    fn test_to_table_with_infinite_ratio() {
        let result = ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Leaf".to_string(),
                file: String::new(),
                entries: vec![BoundaryEntry {
                    incoming: 5,
                    outgoing: 0,
                    ratio: f64::INFINITY,
                }],
                function_count: None,
            }],
        };
        let output = result.to_table();
        assert!(output.contains("\u{221e}"), "Table should show infinity symbol");
        assert!(output.contains("MyApp.Leaf"));
    }
}
