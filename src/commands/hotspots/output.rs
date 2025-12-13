//! Output formatting for hotspots command results.

use super::execute::HotspotEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<HotspotEntry> {
    type Entry = HotspotEntry;

    fn format_header(&self) -> String {
        let kind = self.kind_filter.as_ref().map(|s| s.as_str()).unwrap_or("all");
        let filter_info = if self.module_pattern != "*" {
            format!(" (module: {})", self.module_pattern)
        } else {
            String::new()
        };
        format!("Hotspots ({}){}", kind, filter_info)
    }

    fn format_empty_message(&self) -> String {
        "No hotspots found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} hotspot(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_module_header_with_entries(
        &self,
        module_name: &str,
        _module_file: &str,
        entries: &[HotspotEntry],
    ) -> String {
        if entries.is_empty() {
            return format!("{}:", module_name);
        }

        // Aggregate incoming, outgoing, and total across all entries in the module
        let total_incoming: i64 = entries.iter().map(|e| e.incoming).sum();
        let total_outgoing: i64 = entries.iter().map(|e| e.outgoing).sum();
        let total_total: i64 = entries.iter().map(|e| e.total).sum();

        format!(
            "{}: (in: {}, out: {}, total: {})",
            module_name, total_incoming, total_outgoing, total_total
        )
    }

    fn format_entry(&self, entry: &HotspotEntry, _module: &str, _file: &str) -> String {
        let kind = self.kind_filter.as_ref().map(|s| s.as_str()).unwrap_or("all");
        if kind == "ratio" {
            format!(
                "{} (in: {}, out: {}, ratio: {:.2})",
                entry.function, entry.incoming, entry.outgoing, entry.ratio
            )
        } else {
            format!(
                "{} (in: {}, out: {}, total: {})",
                entry.function, entry.incoming, entry.outgoing, entry.total
            )
        }
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
