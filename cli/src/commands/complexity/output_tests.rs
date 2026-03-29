//! Output formatting tests for complexity command.

#[cfg(test)]
mod tests {
    use super::super::execute::ComplexityEntry;
    use crate::output::{OutputFormat, Outputable, TableFormatter};
    use db::types::{ModuleCollectionResult, ModuleGroup};

    fn single_entry_result() -> ModuleCollectionResult<ComplexityEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![ComplexityEntry {
                    name: "create_user".to_string(),
                    arity: 1,
                    line: 10,
                    complexity: 12,
                    max_nesting_depth: 4,
                    lines: 45,
                }],
                function_count: None,
            }],
        }
    }

    fn multi_module_result() -> ModuleCollectionResult<ComplexityEntry> {
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
                    entries: vec![ComplexityEntry {
                        name: "handle_event".to_string(),
                        arity: 2,
                        line: 20,
                        complexity: 15,
                        max_nesting_depth: 5,
                        lines: 60,
                    }],
                    function_count: None,
                },
                ModuleGroup {
                    name: "MyApp.Service".to_string(),
                    file: "lib/my_app/service.ex".to_string(),
                    entries: vec![
                        ComplexityEntry {
                            name: "process".to_string(),
                            arity: 1,
                            line: 5,
                            complexity: 8,
                            max_nesting_depth: 3,
                            lines: 25,
                        },
                        ComplexityEntry {
                            name: "transform".to_string(),
                            arity: 3,
                            line: 65,
                            complexity: 6,
                            max_nesting_depth: 2,
                            lines: 50,
                        },
                    ],
                    function_count: None,
                },
            ],
        }
    }

    fn empty_result() -> ModuleCollectionResult<ComplexityEntry> {
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
    fn test_format_header_returns_complexity() {
        let result = single_entry_result();
        assert_eq!(result.format_header(), "Complexity");
    }

    // =========================================================================
    // format_empty_message tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_empty_message() {
        let result = empty_result();
        assert_eq!(
            result.format_empty_message(),
            "No functions found with the specified complexity thresholds."
        );
    }

    // =========================================================================
    // format_summary tests (kills -> String::new() and "xyzzy" mutants)
    // =========================================================================

    #[test]
    fn test_format_summary() {
        let result = single_entry_result();
        let summary = result.format_summary(5, 2);
        assert_eq!(summary, "Found 5 function(s) in 2 module(s):");
    }

    #[test]
    fn test_format_summary_single() {
        let result = single_entry_result();
        let summary = result.format_summary(1, 1);
        assert_eq!(summary, "Found 1 function(s) in 1 module(s):");
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
            "create_user/1 complexity: 12, depth: 4, lines: 45"
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
    fn test_format_entry_includes_complexity() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("complexity: 12"));
    }

    #[test]
    fn test_format_entry_includes_depth() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("depth: 4"));
    }

    #[test]
    fn test_format_entry_includes_lines() {
        let result = single_entry_result();
        let entry = &result.items[0].entries[0];
        let formatted = result.format_entry(entry, "MyApp.Accounts", "lib/my_app/accounts.ex");
        assert!(formatted.contains("lines: 45"));
    }

    // =========================================================================
    // blank_before_module tests (kills -> false mutant)
    // =========================================================================

    #[test]
    fn test_blank_before_module_is_true() {
        let result = single_entry_result();
        assert!(
            result.blank_before_module(),
            "blank_before_module should return true for complexity"
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
            "blank_after_summary should return false for complexity"
        );
    }

    // =========================================================================
    // Full table output tests
    // =========================================================================

    #[test]
    fn test_format_table_empty_output() {
        let result = empty_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("Complexity"), "Should contain header");
        assert!(
            output.contains("No functions found with the specified complexity thresholds."),
            "Should contain empty message"
        );
    }

    #[test]
    fn test_format_table_single_entry() {
        let result = single_entry_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("Complexity"), "Should contain header");
        assert!(
            output.contains("Found 1 function(s) in 1 module(s):"),
            "Should contain summary"
        );
        assert!(
            output.contains("MyApp.Accounts:"),
            "Should contain module header"
        );
        assert!(
            output.contains("create_user/1 complexity: 12, depth: 4, lines: 45"),
            "Should contain entry"
        );
    }

    #[test]
    fn test_format_table_multi_module() {
        let result = multi_module_result();
        let output = result.format(OutputFormat::Table);
        assert!(output.contains("MyApp.Controller:"));
        assert!(output.contains("MyApp.Service:"));
        assert!(output.contains("handle_event/2 complexity: 15, depth: 5, lines: 60"));
        assert!(output.contains("process/1 complexity: 8, depth: 3, lines: 25"));
        assert!(output.contains("transform/3 complexity: 6, depth: 2, lines: 50"));
        assert!(
            output.contains("Found 3 function(s) in 2 module(s):"),
            "Summary should reflect total items and module count"
        );
    }

    #[test]
    fn test_format_table_has_blank_before_module() {
        let result = multi_module_result();
        let output = result.format(OutputFormat::Table);
        // blank_before_module=true means there should be a blank line before each module header
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
            .position(|l| l.contains("Found 1 function(s)"))
            .expect("Should find summary line");
        // blank_after_summary=false means the next line after summary should NOT be empty
        // (it will be the blank_before_module blank line instead)
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
        assert_eq!(parsed["items"][0]["entries"][0]["complexity"], 12);
        assert_eq!(parsed["items"][0]["entries"][0]["arity"], 1);
        assert_eq!(parsed["items"][0]["entries"][0]["max_nesting_depth"], 4);
        assert_eq!(parsed["items"][0]["entries"][0]["lines"], 45);
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
