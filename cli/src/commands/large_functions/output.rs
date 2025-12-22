//! Output formatting for large functions command results.

use super::execute::LargeFunctionEntry;
use crate::output::TableFormatter;
use db::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<LargeFunctionEntry> {
    type Entry = LargeFunctionEntry;

    fn format_header(&self) -> String {
        "Large Functions".to_string()
    }

    fn format_empty_message(&self) -> String {
        "No large functions found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} large function(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, entry: &LargeFunctionEntry, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} ({} lines) - {}:{}-{}",
            entry.name, entry.arity, entry.lines, entry.file, entry.start_line, entry.end_line
        )
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
