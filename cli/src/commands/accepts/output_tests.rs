//! Output formatting tests for accepts command.

#[cfg(test)]
mod tests {
    use super::super::execute::AcceptsInfo;
    use db::types::{ModuleGroup, ModuleGroupResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Functions accepting \"integer()\"

No functions found.";

    const SINGLE_TABLE: &str = "\
Functions accepting \"integer()\"

Found 1 function(s) in 1 module(s):

MyApp.Accounts:
  get_user/1 (integer()) \u{2192} {:ok, user()}, {:error, :not_found}";

    const MULTIPLE_TABLE: &str = "\
Functions accepting \"integer()\"

Found 2 function(s) in 1 module(s):

MyApp.Accounts:
  get_user/1 (integer()) \u{2192} {:ok, user()}, {:error, :not_found}
  get_user/2 (integer(), keyword()) \u{2192} {:ok, user()}, {:error, :not_found}";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleGroupResult<AcceptsInfo> {
        ModuleGroupResult {
            module_pattern: "*".to_string(),
            function_pattern: Some("integer()".to_string()),
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<AcceptsInfo> {
        ModuleGroupResult {
            module_pattern: "*".to_string(),
            function_pattern: Some("integer()".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![AcceptsInfo {
                    name: "get_user".to_string(),
                    arity: 1,
                    inputs: "integer()".to_string(),
                    return_type: "{:ok, user()}, {:error, :not_found}".to_string(),
                    line: 10,
                }],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<AcceptsInfo> {
        ModuleGroupResult {
            module_pattern: "*".to_string(),
            function_pattern: Some("integer()".to_string()),
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![
                    AcceptsInfo {
                        name: "get_user".to_string(),
                        arity: 1,
                        inputs: "integer()".to_string(),
                        return_type: "{:ok, user()}, {:error, :not_found}".to_string(),
                        line: 10,
                    },
                    AcceptsInfo {
                        name: "get_user".to_string(),
                        arity: 2,
                        inputs: "integer(), keyword()".to_string(),
                        return_type: "{:ok, user()}, {:error, :not_found}".to_string(),
                        line: 12,
                    },
                ],
                function_count: None,
            }],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: MULTIPLE_TABLE,
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: db::test_utils::load_output_fixture("accepts", "single.json"),
        format: Json,
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: db::test_utils::load_output_fixture("accepts", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<AcceptsInfo>,
        expected: db::test_utils::load_output_fixture("accepts", "empty.toon"),
        format: Toon,
    }

    // =========================================================================
    // Format-specific content assertions
    // =========================================================================

    // These tests kill mutants by verifying specific content in outputs,
    // ensuring format methods return meaningful data rather than empty strings.

    #[rstest]
    fn test_format_header_contains_pattern(single_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::Outputable;
        let table = single_result.to_table();
        assert!(
            table.contains("Functions accepting \"integer()\""),
            "Header should contain the search pattern"
        );
    }

    #[rstest]
    fn test_format_empty_message_content(empty_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::Outputable;
        let table = empty_result.to_table();
        assert!(
            table.contains("No functions found."),
            "Empty result should show 'No functions found.'"
        );
    }

    #[rstest]
    fn test_format_summary_content(multiple_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::Outputable;
        let table = multiple_result.to_table();
        assert!(
            table.contains("Found 2 function(s) in 1 module(s):"),
            "Summary should show correct counts"
        );
    }

    #[rstest]
    fn test_format_module_header_content(single_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::Outputable;
        let table = single_result.to_table();
        assert!(
            table.contains("MyApp.Accounts:"),
            "Module header should contain module name with colon"
        );
    }

    #[rstest]
    fn test_format_entry_content(single_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::Outputable;
        let table = single_result.to_table();
        assert!(
            table.contains("get_user/1 (integer())"),
            "Entry should contain name/arity (inputs)"
        );
        assert!(
            table.contains("\u{2192}"),
            "Entry should contain arrow symbol"
        );
    }

    // =========================================================================
    // run() integration test
    // =========================================================================

    #[rstest]
    fn test_run_produces_output(single_result: ModuleGroupResult<AcceptsInfo>) {
        use crate::output::{OutputFormat, Outputable};

        // Test Table format produces non-empty, meaningful output
        let table_output = single_result.format(OutputFormat::Table);
        assert!(!table_output.is_empty(), "Table output should not be empty");
        assert!(table_output.contains("get_user"), "Table output should contain function names");

        // Test JSON format produces valid JSON
        let json_output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Should produce valid JSON");
        assert_eq!(parsed["function_pattern"], "integer()");
        assert_eq!(parsed["total_items"], 1);

        // Test Toon format produces non-empty output
        let toon_output = single_result.format(OutputFormat::Toon);
        assert!(!toon_output.is_empty(), "Toon output should not be empty");
    }
}
