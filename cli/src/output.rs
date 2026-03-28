//! Output formatting for command results.
//!
//! Supports multiple output formats: table (human-readable), JSON, and toon.

use clap::ValueEnum;
use serde::Serialize;
use db::types::{ModuleGroupResult, ModuleCollectionResult};

/// Output format for command results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format
    Json,
    /// Token-efficient toon format
    Toon,
}

/// Trait for types that can be formatted for output
pub trait Outputable: Serialize {
    /// Format as a human-readable table
    fn to_table(&self) -> String;

    /// Format according to the specified output format
    fn format(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Table => self.to_table(),
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
            OutputFormat::Toon => {
                let json_value = serde_json::to_value(self).unwrap_or_default();
                toon::encode(&json_value, None)
            }
        }
    }
}

/// Trait for customizing table formatting for module-grouped results
///
/// Provides hooks for formatting headers, empty states, summaries, and individual entries.
/// Used as a foundation for generating default `to_table()` implementations for generic
/// result types like `ModuleGroupResult<E>` and `ModuleCollectionResult<E>`.
pub trait TableFormatter {
    type Entry;

    /// Format the header line(s) of the table
    fn format_header(&self) -> String;

    /// Format the message shown when there are no results
    fn format_empty_message(&self) -> String;

    /// Format the summary line after header and before entries
    ///
    /// # Arguments
    /// * `total` - Total number of entries across all modules
    /// * `module_count` - Number of modules in the result
    fn format_summary(&self, total: usize, module_count: usize) -> String;

    /// Format the header for a module
    ///
    /// # Arguments
    /// * `module_name` - Name of the module
    /// * `module_file` - File path associated with the module (may be empty)
    fn format_module_header(&self, module_name: &str, module_file: &str) -> String;

    /// Format the header for a module with access to its entries for aggregation
    ///
    /// Default implementation delegates to `format_module_header`.
    /// Override this to include aggregated data from entries in the module header.
    ///
    /// # Arguments
    /// * `module_name` - Name of the module
    /// * `module_file` - File path associated with the module (may be empty)
    /// * `entries` - Reference to the entries in this module
    fn format_module_header_with_entries(
        &self,
        module_name: &str,
        module_file: &str,
        entries: &[Self::Entry],
    ) -> String {
        let _ = entries; // Silence unused warning for default implementation
        self.format_module_header(module_name, module_file)
    }

    /// Format a single entry within a module
    ///
    /// # Arguments
    /// * `entry` - The entry to format
    /// * `module_name` - Name of the parent module (for context)
    /// * `module_file` - File path of the parent module (for context)
    fn format_entry(&self, entry: &Self::Entry, module_name: &str, module_file: &str) -> String;

    /// Format optional detail lines for an entry
    ///
    /// Default implementation returns empty vec. Override to add details like calls/callers.
    fn format_entry_details(
        &self,
        _entry: &Self::Entry,
        _module_name: &str,
        _module_file: &str,
    ) -> Vec<String> {
        Vec::new()
    }

    /// Whether to add a blank line after the summary
    fn blank_after_summary(&self) -> bool {
        true
    }

    /// Whether to add a blank line before each module header
    fn blank_before_module(&self) -> bool {
        false
    }
}

/// Format module-grouped results as a table.
///
/// This is the shared implementation for both ModuleGroupResult and ModuleCollectionResult.
/// Extracts the common logic to avoid duplication between the two impl blocks.
fn format_module_table<F>(formatter: &F, items: &[db::types::ModuleGroup<F::Entry>], total_items: usize) -> String
where
    F: TableFormatter,
{
    let mut lines = Vec::new();

    lines.push(formatter.format_header());
    lines.push(String::new());

    if items.is_empty() {
        lines.push(formatter.format_empty_message());
        return lines.join("\n");
    }

    lines.push(formatter.format_summary(total_items, items.len()));
    if formatter.blank_after_summary() {
        lines.push(String::new());
    }

    for module in items {
        if formatter.blank_before_module() {
            lines.push(String::new());
        }

        lines.push(formatter.format_module_header_with_entries(
            &module.name,
            &module.file,
            &module.entries,
        ));

        for entry in &module.entries {
            lines.push(format!(
                "  {}",
                formatter.format_entry(entry, &module.name, &module.file)
            ));

            for detail in formatter.format_entry_details(entry, &module.name, &module.file) {
                lines.push(format!("    {}", detail));
            }
        }
    }

    lines.join("\n")
}

/// Default implementation of Outputable for ModuleGroupResult using TableFormatter
impl<E> Outputable for ModuleGroupResult<E>
where
    E: Serialize,
    ModuleGroupResult<E>: TableFormatter<Entry = E>,
{
    fn to_table(&self) -> String {
        format_module_table(self, &self.items, self.total_items)
    }
}

/// Default implementation of Outputable for ModuleCollectionResult using TableFormatter
impl<E> Outputable for ModuleCollectionResult<E>
where
    E: Serialize,
    ModuleCollectionResult<E>: TableFormatter<Entry = E>,
{
    fn to_table(&self) -> String {
        format_module_table(self, &self.items, self.total_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::types::ModuleGroup;

    // -- Helper types and fixtures ------------------------------------------------

    /// Minimal entry type for testing default trait methods
    #[derive(Debug, serde::Serialize)]
    struct TestEntry {
        name: String,
    }

    impl TableFormatter for ModuleGroupResult<TestEntry> {
        type Entry = TestEntry;

        fn format_header(&self) -> String {
            format!("Test: {}", self.module_pattern)
        }

        fn format_empty_message(&self) -> String {
            "No entries.".to_string()
        }

        fn format_summary(&self, total: usize, module_count: usize) -> String {
            format!("Found {} entries in {} modules:", total, module_count)
        }

        fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
            module_name.to_string()
        }

        fn format_entry(&self, entry: &TestEntry, _module_name: &str, _module_file: &str) -> String {
            entry.name.clone()
        }
    }

    /// Entry type that carries detail lines to exercise the format_entry_details path
    #[derive(Debug, serde::Serialize)]
    struct DetailEntry {
        name: String,
        details: Vec<String>,
    }

    /// Formatter that overrides format_entry_details to return non-empty details.
    /// Keeps default blank_after_summary (true) and blank_before_module (false).
    impl TableFormatter for ModuleGroupResult<DetailEntry> {
        type Entry = DetailEntry;

        fn format_header(&self) -> String {
            format!("Detail: {}", self.module_pattern)
        }

        fn format_empty_message(&self) -> String {
            "No detail entries.".to_string()
        }

        fn format_summary(&self, total: usize, module_count: usize) -> String {
            format!("{} detail(s) in {} module(s):", total, module_count)
        }

        fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
            format!("[{}]", module_name)
        }

        fn format_entry(&self, entry: &DetailEntry, _module_name: &str, _module_file: &str) -> String {
            entry.name.clone()
        }

        fn format_entry_details(
            &self,
            entry: &DetailEntry,
            _module_name: &str,
            _module_file: &str,
        ) -> Vec<String> {
            entry.details.clone()
        }
    }

    /// Entry type for testing blank_before_module=true and blank_after_summary=false
    #[derive(Debug, serde::Serialize)]
    struct SpacingEntry {
        name: String,
    }

    impl TableFormatter for ModuleGroupResult<SpacingEntry> {
        type Entry = SpacingEntry;

        fn format_header(&self) -> String {
            "Spacing Header".to_string()
        }

        fn format_empty_message(&self) -> String {
            "No spacing entries.".to_string()
        }

        fn format_summary(&self, total: usize, module_count: usize) -> String {
            format!("{} item(s) in {} module(s):", total, module_count)
        }

        fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
            format!("<{}>", module_name)
        }

        fn format_entry(&self, entry: &SpacingEntry, _module_name: &str, _module_file: &str) -> String {
            entry.name.clone()
        }

        fn blank_after_summary(&self) -> bool {
            false
        }

        fn blank_before_module(&self) -> bool {
            true
        }
    }

    /// TableFormatter impl for ModuleCollectionResult so the Outputable blanket impl is exercised
    impl TableFormatter for ModuleCollectionResult<TestEntry> {
        type Entry = TestEntry;

        fn format_header(&self) -> String {
            format!("Collection: {}", self.module_pattern)
        }

        fn format_empty_message(&self) -> String {
            "No collection entries.".to_string()
        }

        fn format_summary(&self, total: usize, module_count: usize) -> String {
            format!("Collected {} in {} modules:", total, module_count)
        }

        fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
            module_name.to_string()
        }

        fn format_entry(&self, entry: &TestEntry, _module_name: &str, _module_file: &str) -> String {
            entry.name.clone()
        }
    }

    /// Convenience builder for a single-module ModuleGroupResult<TestEntry>
    fn single_module_result() -> ModuleGroupResult<TestEntry> {
        ModuleGroupResult {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        }
    }

    fn empty_result() -> ModuleGroupResult<TestEntry> {
        ModuleGroupResult {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        }
    }

    // -- Default trait method tests -----------------------------------------------

    #[test]
    fn test_format_entry_details_default_returns_empty_vec() {
        let result = single_module_result();
        let details = result.format_entry_details(
            &result.items[0].entries[0],
            "TestModule",
            "test.ex",
        );
        assert!(details.is_empty(), "Default format_entry_details should return empty vec");
    }

    #[test]
    fn test_format_module_header_with_entries_default_delegates() {
        let result = single_module_result();
        let header_with = result.format_module_header_with_entries(
            "TestModule",
            "test.ex",
            &result.items[0].entries,
        );
        let header_without = result.format_module_header("TestModule", "test.ex");
        assert_eq!(
            header_with, header_without,
            "Default format_module_header_with_entries should delegate to format_module_header"
        );
    }

    #[test]
    fn test_blank_after_summary_default() {
        let result = empty_result();
        assert!(result.blank_after_summary(), "Default blank_after_summary should return true");
    }

    #[test]
    fn test_blank_before_module_default() {
        let result = empty_result();
        assert!(!result.blank_before_module(), "Default blank_before_module should return false");
    }

    // -- format_module_table tests (via to_table) ---------------------------------

    #[test]
    fn test_format_module_table_empty() {
        let result = empty_result();
        let table = result.to_table();
        assert!(table.contains("No entries."), "Empty result should show empty message");
        // Empty path should NOT include the summary line
        assert!(
            !table.contains("Found"),
            "Empty result should not contain a summary line"
        );
    }

    #[test]
    fn test_format_module_table_without_details() {
        let result = single_module_result();
        let table = result.to_table();
        assert!(table.contains("  entry1"), "Should contain 2-space-indented entry");
        assert!(!table.contains("    "), "Should not contain detail lines (4-space indent)");
    }

    #[test]
    fn test_format_module_table_includes_header_and_summary() {
        let result = single_module_result();
        let table = result.to_table();
        assert!(table.contains("Test: test"), "Should contain the header");
        assert!(
            table.contains("Found 1 entries in 1 modules:"),
            "Should contain the summary"
        );
        assert!(table.contains("TestModule"), "Should contain the module header");
    }

    #[test]
    fn test_format_module_table_with_details() {
        let result = ModuleGroupResult::<DetailEntry> {
            module_pattern: "det".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "Mod".to_string(),
                file: "mod.ex".to_string(),
                entries: vec![DetailEntry {
                    name: "fn1".to_string(),
                    details: vec!["arg: integer".to_string(), "returns: bool".to_string()],
                }],
                function_count: None,
            }],
        };

        let table = result.to_table();
        assert!(table.contains("  fn1"), "Should contain 2-space-indented entry");
        assert!(
            table.contains("    arg: integer"),
            "Should contain 4-space-indented detail line"
        );
        assert!(
            table.contains("    returns: bool"),
            "Should contain second detail line"
        );
    }

    #[test]
    fn test_format_module_table_blank_before_module() {
        // SpacingEntry formatter sets blank_before_module -> true
        let result = ModuleGroupResult::<SpacingEntry> {
            module_pattern: "sp".to_string(),
            function_pattern: None,
            total_items: 2,
            items: vec![
                ModuleGroup {
                    name: "ModA".to_string(),
                    file: "a.ex".to_string(),
                    entries: vec![SpacingEntry { name: "fn_a".to_string() }],
                    function_count: None,
                },
                ModuleGroup {
                    name: "ModB".to_string(),
                    file: "b.ex".to_string(),
                    entries: vec![SpacingEntry { name: "fn_b".to_string() }],
                    function_count: None,
                },
            ],
        };

        let table = result.to_table();
        let lines: Vec<&str> = table.lines().collect();

        // blank_before_module inserts an empty line before each module header.
        let mod_a_idx = lines.iter().position(|l| l.contains("<ModA>")).expect("Should contain <ModA>");
        let mod_b_idx = lines.iter().position(|l| l.contains("<ModB>")).expect("Should contain <ModB>");
        assert_eq!(lines[mod_a_idx - 1], "", "Blank line should precede first module header");
        assert_eq!(lines[mod_b_idx - 1], "", "Blank line should precede second module header");
    }

    #[test]
    fn test_format_module_table_no_blank_after_summary() {
        // SpacingEntry formatter sets blank_after_summary -> false
        // Use a single module so blank_before_module's blank line is clearly distinguishable
        let result = ModuleGroupResult::<SpacingEntry> {
            module_pattern: "sp".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "Mod".to_string(),
                file: "mod.ex".to_string(),
                entries: vec![SpacingEntry { name: "fn1".to_string() }],
                function_count: None,
            }],
        };

        let table = result.to_table();
        let lines: Vec<&str> = table.lines().collect();

        let summary_idx = lines
            .iter()
            .position(|l| l.contains("item(s) in"))
            .expect("Should contain summary line");
        // blank_after_summary==false means no blank line from that flag.
        // blank_before_module==true adds a blank line before the module header.
        // So immediately after summary we should see the blank line from blank_before_module,
        // then the module header. With only blank_after_summary inserting, we'd see TWO blank
        // lines (one from each). Verify exactly one blank line separates summary and module.
        let mod_idx = lines
            .iter()
            .position(|l| l.contains("<Mod>"))
            .expect("Should contain module header");
        assert_eq!(
            mod_idx - summary_idx,
            2,
            "Exactly one blank line (from blank_before_module) should separate summary and module header"
        );
        assert_eq!(lines[summary_idx + 1], "", "The separator should be a blank line");
    }

    #[test]
    fn test_format_module_table_blank_after_summary_true() {
        // Default TestEntry formatter has blank_after_summary -> true, blank_before_module -> false
        let result = single_module_result();
        let table = result.to_table();
        let lines: Vec<&str> = table.lines().collect();

        let summary_idx = lines
            .iter()
            .position(|l| l.contains("Found"))
            .expect("Should contain summary line");
        // blank_after_summary==true should insert a blank line after summary
        assert_eq!(
            lines[summary_idx + 1], "",
            "Blank line should follow summary when blank_after_summary is true"
        );
        // blank_before_module==false means the module header follows immediately after the blank
        assert!(
            lines[summary_idx + 2].contains("TestModule"),
            "Module header should follow the blank line"
        );
    }

    #[test]
    fn test_format_module_table_multiple_modules() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "multi".to_string(),
            function_pattern: None,
            total_items: 3,
            items: vec![
                ModuleGroup {
                    name: "Alpha".to_string(),
                    file: "alpha.ex".to_string(),
                    entries: vec![
                        TestEntry { name: "a1".to_string() },
                        TestEntry { name: "a2".to_string() },
                    ],
                    function_count: None,
                },
                ModuleGroup {
                    name: "Beta".to_string(),
                    file: "beta.ex".to_string(),
                    entries: vec![TestEntry { name: "b1".to_string() }],
                    function_count: None,
                },
            ],
        };

        let table = result.to_table();
        assert!(table.contains("Found 3 entries in 2 modules:"), "Summary should reflect totals");
        assert!(table.contains("Alpha"), "Should contain first module");
        assert!(table.contains("Beta"), "Should contain second module");
        assert!(table.contains("  a1"), "Should contain first module's first entry");
        assert!(table.contains("  a2"), "Should contain first module's second entry");
        assert!(table.contains("  b1"), "Should contain second module's entry");
    }

    // -- ModuleCollectionResult::to_table -----------------------------------------

    #[test]
    fn test_module_collection_result_to_table_empty() {
        let result = ModuleCollectionResult::<TestEntry> {
            module_pattern: "coll".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        };

        let table = result.to_table();
        assert!(
            table.contains("No collection entries."),
            "Empty collection should show empty message"
        );
    }

    #[test]
    fn test_module_collection_result_to_table_with_entries() {
        let result = ModuleCollectionResult::<TestEntry> {
            module_pattern: "coll".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "CollModule".to_string(),
                file: "coll.ex".to_string(),
                entries: vec![TestEntry { name: "c1".to_string() }],
                function_count: None,
            }],
        };

        let table = result.to_table();
        assert!(table.contains("Collection: coll"), "Should contain header");
        assert!(table.contains("Collected 1 in 1 modules:"), "Should contain summary");
        assert!(table.contains("CollModule"), "Should contain module name");
        assert!(table.contains("  c1"), "Should contain entry");
    }

    // -- Outputable::format dispatch tests ----------------------------------------

    #[test]
    fn test_format_table_dispatches_to_to_table() {
        let result = single_module_result();
        let via_format = result.format(OutputFormat::Table);
        // Rebuild an identical result to call to_table directly
        let result2 = single_module_result();
        let via_to_table = result2.to_table();
        assert_eq!(
            via_format, via_to_table,
            "format(Table) should produce the same output as to_table()"
        );
    }

    #[test]
    fn test_format_json_produces_valid_json() {
        let result = single_module_result();
        let json = result.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("Should produce valid JSON");
        assert_eq!(parsed["module_pattern"], "test");
        assert_eq!(parsed["total_items"], 1);
        assert_eq!(parsed["items"][0]["name"], "TestModule");
    }

    #[test]
    fn test_format_toon_contains_field_values() {
        let result = single_module_result();
        let toon = result.format(OutputFormat::Toon);
        assert!(!toon.is_empty(), "Toon output should not be empty");
        // Toon is a compact text encoding; verify key data is present
        assert!(toon.contains("test"), "Toon should contain the module_pattern value");
        assert!(toon.contains("TestModule"), "Toon should contain the module name");
    }
}
