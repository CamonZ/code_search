//! Output formatting for hotspots command results.

use super::execute::HotspotsResult;
use crate::output::Outputable;

impl Outputable for HotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let filter_info = match &self.module_filter {
            Some(pattern) => format!(" (module filter: {})", pattern),
            None => String::new(),
        };
        lines.push(format!(
            "Hotspots ({}) in project '{}'{}",
            self.kind, self.project, filter_info
        ));
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No hotspots found.".to_string());
        } else {
            lines.push(format!(
                "Found {} hotspot(s) in {} module(s):",
                self.total_hotspots,
                self.modules.len()
            ));

            for module in &self.modules {
                lines.push(String::new());
                lines.push(format!("{}:", module.name));

                for entry in &module.functions {
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
