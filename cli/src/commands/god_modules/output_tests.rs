//! Output formatting tests for god_modules command.

#[cfg(test)]
mod tests {
    use super::super::execute::GodModuleEntry;
    use crate::output::{OutputFormat, Outputable};
    use db::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
God Modules

No god modules found.";

    // Note: Each module has one entry where format_entry returns "",
    // so the shared formatter emits "  " (indent + empty string) after each module header.
    const SINGLE_TABLE: &str = "God Modules\n\nFound 1 god module(s) in 1 module(s):\n\nMyApp.Core: (funcs: 25, loc: 800, in: 10, out: 5, total: 15)\n  ";

    const MULTIPLE_TABLE: &str = "God Modules\n\nFound 2 god module(s) in 2 module(s):\n\nMyApp.Core: (funcs: 25, loc: 800, in: 10, out: 5, total: 15)\n  \n\nMyApp.Users: (funcs: 20, loc: 600, in: 8, out: 4, total: 12)\n  ";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<GodModuleEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("god".to_string()),
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<GodModuleEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("god".to_string()),
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Core".to_string(),
                file: String::new(),
                entries: vec![GodModuleEntry {
                    function_count: 25,
                    loc: 800,
                    incoming: 10,
                    outgoing: 5,
                    total: 15,
                }],
                function_count: Some(25),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleCollectionResult<GodModuleEntry> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: Some("god".to_string()),
            name_filter: None,
            total_items: 2,
            items: vec![
                ModuleGroup {
                    name: "MyApp.Core".to_string(),
                    file: String::new(),
                    entries: vec![GodModuleEntry {
                        function_count: 25,
                        loc: 800,
                        incoming: 10,
                        outgoing: 5,
                        total: 15,
                    }],
                    function_count: Some(25),
                },
                ModuleGroup {
                    name: "MyApp.Users".to_string(),
                    file: String::new(),
                    entries: vec![GodModuleEntry {
                        function_count: 20,
                        loc: 600,
                        incoming: 8,
                        outgoing: 4,
                        total: 12,
                    }],
                    function_count: Some(20),
                },
            ],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    #[rstest]
    fn test_to_table_empty(empty_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_to_table_single(single_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = single_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = multiple_result.to_table();
        assert_eq!(output, MULTIPLE_TABLE);
    }

    // =========================================================================
    // Individual formatter tests (kill specific output.rs mutants)
    // =========================================================================

    /// Tests format_header returns "God Modules" exactly.
    /// Kills: output.rs:11 format_header -> String::new() and "xyzzy"
    #[rstest]
    fn test_format_header(empty_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert_eq!(empty_result.format_header(), "God Modules");
    }

    /// Tests format_empty_message returns the exact empty message.
    /// Kills: output.rs:15 format_empty_message -> String::new() and "xyzzy"
    #[rstest]
    fn test_format_empty_message(empty_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert_eq!(
            empty_result.format_empty_message(),
            "No god modules found."
        );
    }

    /// Tests format_summary returns the correct formatted summary.
    /// Kills: output.rs:19 format_summary -> String::new() and "xyzzy"
    #[rstest]
    fn test_format_summary(single_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert_eq!(
            single_result.format_summary(3, 2),
            "Found 3 god module(s) in 2 module(s):"
        );
    }

    /// Tests format_module_header returns "module_name:" format.
    /// Kills: output.rs:23 format_module_header -> String::new() and "xyzzy"
    #[rstest]
    fn test_format_module_header(single_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert_eq!(
            single_result.format_module_header("MyApp.Core", ""),
            "MyApp.Core:"
        );
    }

    /// Tests format_module_header_with_entries returns stats-enriched header.
    /// Kills: output.rs:32 format_module_header_with_entries -> String::new() and "xyzzy"
    #[rstest]
    fn test_format_module_header_with_entries(
        single_result: ModuleCollectionResult<GodModuleEntry>,
    ) {
        use crate::output::TableFormatter;
        let entries = vec![GodModuleEntry {
            function_count: 25,
            loc: 800,
            incoming: 10,
            outgoing: 5,
            total: 15,
        }];
        assert_eq!(
            single_result.format_module_header_with_entries("MyApp.Core", "", &entries),
            "MyApp.Core: (funcs: 25, loc: 800, in: 10, out: 5, total: 15)"
        );
    }

    /// Tests format_module_header_with_entries with empty entries falls back.
    #[rstest]
    fn test_format_module_header_with_empty_entries(
        single_result: ModuleCollectionResult<GodModuleEntry>,
    ) {
        use crate::output::TableFormatter;
        let entries: Vec<GodModuleEntry> = vec![];
        assert_eq!(
            single_result.format_module_header_with_entries("MyApp.Core", "", &entries),
            "MyApp.Core:"
        );
    }

    /// Tests format_entry returns empty string (god modules show stats in module header).
    /// Kills: output.rs:45 format_entry -> "xyzzy"
    #[rstest]
    fn test_format_entry(single_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        let entry = GodModuleEntry {
            function_count: 25,
            loc: 800,
            incoming: 10,
            outgoing: 5,
            total: 15,
        };
        assert_eq!(
            single_result.format_entry(&entry, "MyApp.Core", ""),
            String::new()
        );
    }

    /// Tests blank_before_module returns true.
    /// Kills: output.rs:49 blank_before_module -> false
    #[rstest]
    fn test_blank_before_module(single_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert!(
            single_result.blank_before_module(),
            "blank_before_module should return true"
        );
    }

    /// Tests blank_after_summary returns false.
    /// Kills: output.rs:53 blank_after_summary -> true
    #[rstest]
    fn test_blank_after_summary(single_result: ModuleCollectionResult<GodModuleEntry>) {
        use crate::output::TableFormatter;
        assert!(
            !single_result.blank_after_summary(),
            "blank_after_summary should return false"
        );
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_format_json(single_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = single_result.format(OutputFormat::Json);
        assert!(output.contains("\"kind_filter\": \"god\""));
        assert!(output.contains("\"total_items\": 1"));
        assert!(output.contains("\"name\": \"MyApp.Core\""));
        assert!(output.contains("\"function_count\": 25"));
        assert!(output.contains("\"loc\": 800"));
        assert!(output.contains("\"incoming\": 10"));
        assert!(output.contains("\"outgoing\": 5"));
        assert!(output.contains("\"total\": 15"));
    }

    #[rstest]
    fn test_format_json_empty(empty_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = empty_result.format(OutputFormat::Json);
        assert!(output.contains("\"kind_filter\": \"god\""));
        assert!(output.contains("\"total_items\": 0"));
        assert!(output.contains("\"items\": []"));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_format_toon(single_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("kind_filter"));
        assert!(output.contains("total_items"));
        assert!(output.contains("items"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: ModuleCollectionResult<GodModuleEntry>) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("kind_filter"));
        assert!(output.contains("items"));
    }
}
