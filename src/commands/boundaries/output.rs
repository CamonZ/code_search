//! Output formatting for boundaries command results.

use super::execute::BoundaryEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<BoundaryEntry> {
    type Entry = BoundaryEntry;

    fn format_header(&self) -> String {
        let filter_info = if self.module_pattern != "*" {
            format!(" (module: {})", self.module_pattern)
        } else {
            String::new()
        };
        format!("Boundary Modules{}", filter_info)
    }

    fn format_empty_message(&self) -> String {
        "No boundary modules found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} boundary module(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_module_header_with_entries(
        &self,
        module_name: &str,
        _module_file: &str,
        entries: &[BoundaryEntry],
    ) -> String {
        if entries.is_empty() {
            return format!("{}:", module_name);
        }

        // Get the first (and typically only) entry for module-level stats
        let entry = &entries[0];

        // Format ratio with special case for infinite (outgoing = 0)
        let ratio_str = if entry.outgoing == 0 {
            "∞".to_string()
        } else {
            format!("{:.1}", entry.ratio)
        };

        format!(
            "{}: (in: {}, out: {}, ratio: {})",
            module_name, entry.incoming, entry.outgoing, ratio_str
        )
    }

    fn format_entry(&self, entry: &BoundaryEntry, _module: &str, _file: &str) -> String {
        // For boundaries, we don't show individual entries since there's only one per module
        // But if there are multiple, format them
        let ratio_str = if entry.outgoing == 0 {
            "∞".to_string()
        } else {
            format!("{:.1}", entry.ratio)
        };

        format!(
            "(in: {}, out: {}, ratio: {})",
            entry.incoming, entry.outgoing, ratio_str
        )
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
