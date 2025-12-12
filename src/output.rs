//! Output formatting for command results.
//!
//! Supports multiple output formats: table (human-readable), JSON, and toon.

use clap::ValueEnum;
use serde::Serialize;
use crate::types::{ModuleGroupResult, ModuleCollectionResult};

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

/// Default implementation of Outputable for ModuleGroupResult using TableFormatter
impl<E> Outputable for ModuleGroupResult<E>
where
    E: Serialize,
    ModuleGroupResult<E>: TableFormatter<Entry = E>,
{
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(self.format_header());
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push(self.format_empty_message());
            return lines.join("\n");
        }

        lines.push(self.format_summary(self.total_items, self.items.len()));
        if self.blank_after_summary() {
            lines.push(String::new());
        }

        for module in &self.items {
            if self.blank_before_module() {
                lines.push(String::new());
            }

            lines.push(self.format_module_header(&module.name, &module.file));

            for entry in &module.entries {
                lines.push(format!("  {}", self.format_entry(entry, &module.name, &module.file)));

                for detail in self.format_entry_details(entry, &module.name, &module.file) {
                    lines.push(format!("    {}", detail));
                }
            }
        }

        lines.join("\n")
    }
}

/// Default implementation of Outputable for ModuleCollectionResult using TableFormatter
impl<E> Outputable for ModuleCollectionResult<E>
where
    E: Serialize,
    ModuleCollectionResult<E>: TableFormatter<Entry = E>,
{
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(self.format_header());
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push(self.format_empty_message());
            return lines.join("\n");
        }

        lines.push(self.format_summary(self.total_items, self.items.len()));
        if self.blank_after_summary() {
            lines.push(String::new());
        }

        for module in &self.items {
            if self.blank_before_module() {
                lines.push(String::new());
            }

            lines.push(self.format_module_header(&module.name, &module.file));

            for entry in &module.entries {
                lines.push(format!("  {}", self.format_entry(entry, &module.name, &module.file)));

                for detail in self.format_entry_details(entry, &module.name, &module.file) {
                    lines.push(format!("    {}", detail));
                }
            }
        }

        lines.join("\n")
    }
}
