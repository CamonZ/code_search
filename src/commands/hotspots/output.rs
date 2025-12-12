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

    fn format_entry(&self, entry: &HotspotEntry, _module: &str, _file: &str) -> String {
        format!(
            "{} (in: {}, out: {}, total: {})",
            entry.function, entry.incoming, entry.outgoing, entry.total
        )
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
