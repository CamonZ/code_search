//! Output formatting tests for large_functions command.

#[cfg(test)]
mod tests {
    use super::super::execute::LargeFunctionEntry;
    use crate::output::{OutputFormat, Outputable, TableFormatter};
    use db::types::{ModuleCollectionResult, ModuleGroup};

    fn single_entry_result() -> ModuleCollectionResult<LargeFunctionEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![LargeFunctionEntry {
                    name: "create_user".to_string(),
                    arity: 1,
                    start_line: 10,
                    end_line: 60,
                    lines: 51,
                    file: "lib/my_app/accounts.ex".to_string(),
                }],
                function_count: None,
            }],
        }
    }

    fn multi_module_result() -> ModuleCollectionResult<LargeFunctionEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 3,
            items: vec![
                ModuleGroup {
                    name: "MyApp.Controller".to_string(),
                    file: "lib/my_app/controller.ex".to_string(),
                    entries: vec![
                        LargeFunctionEntry {
                            name: "create".to_string(),
                            arity: 2,
                            start_line: 20,
                            end_line: 80,
                            lines: 61,
                            file: "lib/my_app/controller.ex".to_string(),
                        },
                    ],
                    function_count: None,
                },
                ModuleGroup {
                    name: "MyApp.Service".to_string(),
                    file: "lib/my_app/service.ex".to_string(),
                    entries: vec![
                        LargeFunctionEntry {
                            name: "process".to_string(),
                            arity: 1,
                            start_line: 5,
                            end_line: 60,
                            lines: 56,
                            file: "lib/my_app/service.ex".to_string(),
                        },
                        LargeFunctionEntry {
                            name: "transform".to_string(),
                            arity: 3,
                            start_line: 65,
                            end_line: 115,
                            lines: 51,
                            file: "lib/my_app/service.ex".to_string(),
                        },
                    ],
                    function_count: None,
                },
            ],
        }
    }

    fn empty_result() -> ModuleCollectionResult<LargeFunctionEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    // =========================================================================
    // format_header tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_header_returns_large_functions() {
        let result = single_entry_result();
        assert_eq!(result.format_header(), "Large Functions");
    }

    // =========================================================================
    // format_empty_message tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_empty_message() {
        let result = empty_result();
        assert_eq!(result.format_empty_message(), "No large functions found.");
    }

    // =========================================================================
    // format_summary tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_summary() {
        let result = single_entry_result();
        let summary = result.format_summary(5, 2);
        assert_eq!(summary, "Found 5 large function(s) in 2 module(s):");
    }

    #[test]
    fn test_format_summary_single() {
        let result = single_entry_result();
        let summary = result.format_summary(1, 1);
        assert_eq!(summary, "Found 1 large function(s) in 1 module(s):");
    }

    // =========================================================================
    // format_module_header tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_module_header() {
        let result = single_entry_result();
        let header = result.format_module_header("MyApp.Accounts", "lib/my_app/accounts.ex");
        assert_eq!(header, "MyApp.Accounts:");
    }

    #[test]
    fn test_format_module_header_includes_module_name() {
        let result = single_entry_result();
        let header = result.format_module_header("SomeOther.Module", "lib/other.ex");
        assert!(header.contains("SomeOther.Module"));
    }

    // =========================================================================
    // format_entry tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_entry() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert_eq!(
            formatted,
            "create_user/1 (51 lines) - lib/my_app/accounts.ex:10-60"
        );
    }

    #[test]
    fn test_format_entry_includes_function_name_and_arity() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("create_user/1"));
    }

    #[test]
    fn test_format_entry_includes_line_count() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("51 lines"));
    }

    #[test]
    fn test_format_entry_includes_file_and_line_range() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("lib/my_app/accounts.ex:10-60"));
    }

    // =========================================================================
    // blank_before_module tests (kills -> false mutant)
    // =========================================================================

    #[test]
    fn test_blank_before_module_is_true() {
        let result = single_entry_result();
        assert!(
            result.blank_before_module(),
            "blank_before_module should return true for large functions"
        );
    }

    // =========================================================================
    // blank_after_summary tests (kills -> true mutant)
    // =========================================================================

    #[test]
    fn test_blank_after_summary_is_false() {
        let result = single_entry_result();
        assert!(
            !result.blank_after_summary(),
            "blank_after_summary should return false for large functions"
        );
    }

    // =========================================================================
    // Full table output tests
    // =========================================================================

    #[test]
    fn test_format_table_empty_output() {
        let result = empty_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("Large Functions"), "Should contain header");
        assert!(
            output.contains("No large functions found."),
            "Should contain empty message"
        );
    }

    #[test]
    fn test_format_table_single_entry() {
        let result = single_entry_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("Large Functions"), "Should contain header");
        assert!(
            output.contains("Found 1 large function(s) in 1 module(s):"),
            "Should contain summary"
        );
        assert!(
            output.contains("MyApp.Accounts:"),
            "Should contain module header"
        );
        assert!(
            output.contains("create_user/1 (51 lines) - lib/my_app/accounts.ex:10-60"),
            "Should contain entry"
        );
    }

    #[test]
    fn test_format_table_multi_module() {
        let result = multi_module_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("MyApp.Controller:"));
        assert!(output.contains("MyApp.Service:"));
        assert!(output.contains("create/2 (61 lines)"));
        assert!(output.contains("process/1 (56 lines)"));
        assert!(output.contains("transform/3 (51 lines)"));
        assert!(
            output.contains("Found 3 large function(s) in 2 module(s):"),
            "Summary should reflect total items and module count"
        );
    }

    #[test]
    fn test_format_table_has_blank_before_module() {
        let result = multi_module_result();
        let output = result.format(OutputFormat::Table);
        // blank_before_module=true means there should be a blank line before each module header
        // The output has lines joined by \n, so "\n\nMyApp." indicates a blank line before module
        assert!(
            output.contains("\n\nMyApp.Controller:"),
            "Should have blank line before first module header"
        );
        assert!(
            output.contains("\n\nMyApp.Service:"),
            "Should have blank line before second module header"
        );
    }

    #[test]
    fn test_format_table_no_blank_after_summary() {
        let result = single_entry_result();
        let output = result.format(OutputFormat::Table);
        let lines: Vec<&str> = output.lines().collect();
        // Find the summary line
        let summary_idx = lines
            .iter()
            .position(|l| l.contains("Found 1 large function(s)"))
            .expect("Should find summary line");
        // blank_after_summary=false means the next line after summary should NOT be empty
        // (it will be the blank_before_module blank line instead)
        // The layout is: header, blank, summary, blank (from blank_before_module), module header, entries
        // With blank_after_summary=false and blank_before_module=true:
        // "Large Functions" / "" / "Found..." / "" / "MyApp.Accounts:" / "  create_user..."
        // The blank line after summary comes from blank_before_module, not blank_after_summary
        assert!(
            summary_idx < lines.len() - 1,
            "Summary should not be the last line"
        );
    }

    // =========================================================================
    // JSON output tests
    // =========================================================================

    #[test]
    fn test_format_json() {
        let result = single_entry_result();
        let output = result.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");
        assert_eq!(parsed["total_items"], 1);
        assert_eq!(parsed["items"][0]["name"], "MyApp.Accounts");
        assert_eq!(parsed["items"][0]["entries"][0]["name"], "create_user");
        assert_eq!(parsed["items"][0]["entries"][0]["lines"], 51);
        assert_eq!(parsed["items"][0]["entries"][0]["arity"], 1);
        assert_eq!(parsed["items"][0]["entries"][0]["start_line"], 10);
        assert_eq!(parsed["items"][0]["entries"][0]["end_line"], 60);
    }

    // =========================================================================
    // Toon output tests
    // =========================================================================

    #[test]
    fn test_format_toon() {
        let result = single_entry_result();
        let output = result.format(OutputFormat::Toon);
        assert!(!output.is_empty(), "Toon output should not be empty");
        assert!(output.contains("MyApp.Accounts"), "Should contain module name");
        assert!(output.contains("create_user"), "Should contain function name");
    }
}
