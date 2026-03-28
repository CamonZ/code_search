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

    // A minimal entry type for testing the default trait methods
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

    #[test]
    fn test_format_entry_details_default_returns_empty_vec() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        };

        let details = result.format_entry_details(
            &result.items[0].entries[0],
            "TestModule",
            "test.ex",
        );
        assert!(details.is_empty(), "Default format_entry_details should return empty vec");
    }

    #[test]
    fn test_format_module_table_without_details() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        };

        let table = result.to_table();
        // The output should contain the entry but no detail lines (indented with 4 spaces)
        assert!(table.contains("  entry1"), "Should contain entry");
        assert!(!table.contains("    "), "Should not contain detail lines (4-space indent)");
    }

    #[test]
    fn test_format_module_header_with_entries_default_delegates() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        };

        let header_with = result.format_module_header_with_entries(
            "TestModule",
            "test.ex",
            &result.items[0].entries,
        );
        let header_without = result.format_module_header("TestModule", "test.ex");
        assert_eq!(header_with, header_without, "Default format_module_header_with_entries should delegate to format_module_header");
    }

    #[test]
    fn test_blank_after_summary_default() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        };
        assert!(result.blank_after_summary(), "Default blank_after_summary should return true");
    }

    #[test]
    fn test_blank_before_module_default() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        };
        assert!(!result.blank_before_module(), "Default blank_before_module should return false");
    }

    #[test]
    fn test_format_module_table_empty() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        };

        let table = result.to_table();
        assert!(table.contains("No entries."), "Empty result should show empty message");
    }

    #[test]
    fn test_format_output_json() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        };

        let json = result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should produce valid JSON");
        assert_eq!(parsed["module_pattern"], "test");
    }

    #[test]
    fn test_format_output_toon() {
        let result = ModuleGroupResult::<TestEntry> {
            module_pattern: "test".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "TestModule".to_string(),
                file: "test.ex".to_string(),
                entries: vec![TestEntry { name: "entry1".to_string() }],
                function_count: None,
            }],
        };

        let toon = result.format(OutputFormat::Toon);
        assert!(!toon.is_empty(), "Toon output should not be empty");
    }
}
