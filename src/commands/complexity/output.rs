//! Output formatting for complexity command results.

use super::execute::ComplexityEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<ComplexityEntry> {
    type Entry = ComplexityEntry;

    fn format_header(&self) -> String {
        "Complexity".to_string()
    }

    fn format_empty_message(&self) -> String {
        "No functions found with the specified complexity thresholds.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} function(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, entry: &ComplexityEntry, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} complexity: {}, depth: {}, lines: {}",
            entry.name, entry.arity, entry.complexity, entry.max_nesting_depth, entry.lines
        )
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
