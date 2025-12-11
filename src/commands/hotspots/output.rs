//! Output formatting for hotspots command results.

use super::execute::HotspotEntry;
use crate::output::Outputable;
use crate::types::ModuleCollectionResult;

impl Outputable for ModuleCollectionResult<HotspotEntry> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let kind = self.kind_filter.as_ref().map(|s| s.as_str()).unwrap_or("all");
        let filter_info = if self.module_pattern != "*" {
            format!(" (module: {})", self.module_pattern)
        } else {
            String::new()
        };
        lines.push(format!("Hotspots ({}){}", kind, filter_info));
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push("No hotspots found.".to_string());
        } else {
            lines.push(format!(
                "Found {} hotspot(s) in {} module(s):",
                self.total_items,
                self.items.len()
            ));

            for module in &self.items {
                lines.push(String::new());
                lines.push(format!("{}:", module.name));

                for entry in &module.entries {
                    lines.push(format!(
                        "  {} (in: {}, out: {}, total: {})",
                        entry.function, entry.incoming, entry.outgoing, entry.total
                    ));
                }
            }
        }

        lines.join("\n")
    }
}
