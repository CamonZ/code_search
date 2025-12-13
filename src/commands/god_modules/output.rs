//! Output formatting for god modules command results.

use super::execute::GodModuleEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<GodModuleEntry> {
    type Entry = GodModuleEntry;

    fn format_header(&self) -> String {
        "God Modules".to_string()
    }

    fn format_empty_message(&self) -> String {
        "No god modules found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} god module(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_module_header_with_entries(
        &self,
        module_name: &str,
        _module_file: &str,
        entries: &[GodModuleEntry],
    ) -> String {
        if let Some(entry) = entries.first() {
            format!(
                "{}: (funcs: {}, in: {}, out: {}, total: {})",
                module_name, entry.function_count, entry.incoming, entry.outgoing, entry.total
            )
        } else {
            format!("{}:", module_name)
        }
    }

    fn format_entry(&self, _entry: &GodModuleEntry, _module: &str, _file: &str) -> String {
        // For god modules, we don't show individual entries since each module has only one entry
        // The module header already shows all the stats
        String::new()
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
