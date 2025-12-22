//! Output formatting for many clauses command results.

use super::execute::ManyClausesEntry;
use crate::output::TableFormatter;
use db::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<ManyClausesEntry> {
    type Entry = ManyClausesEntry;

    fn format_header(&self) -> String {
        "Functions with Many Clauses".to_string()
    }

    fn format_empty_message(&self) -> String {
        "No functions with many clauses found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} function(s) with many clauses in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, entry: &ManyClausesEntry, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} ({} clauses) - {}:{}-{}",
            entry.name, entry.arity, entry.clauses, entry.file, entry.first_line, entry.last_line
        )
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
