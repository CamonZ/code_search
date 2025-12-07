//! Output formatting for hotspots command results.

use crate::output::Outputable;
use super::execute::HotspotsResult;

impl Outputable for HotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let filter_info = match &self.module_filter {
            Some(pattern) => format!(" (module filter: {})", pattern),
            None => String::new(),
        };
        lines.push(format!("Hotspots ({}) in project '{}'{}", self.kind, self.project, filter_info));
        lines.push(String::new());

        if !self.hotspots.is_empty() {
            // Header
            lines.push(format!("{:<50} {:>8} {:>8} {:>8}", "FUNCTION", "IN", "OUT", "TOTAL"));
            lines.push("-".repeat(78));

            for hotspot in &self.hotspots {
                let sig = format!("{}.{}", hotspot.module, hotspot.function);
                lines.push(format!(
                    "{:<50} {:>8} {:>8} {:>8}",
                    sig, hotspot.incoming, hotspot.outgoing, hotspot.total
                ));
            }
        } else {
            lines.push("No hotspots found.".to_string());
        }

        lines.join("\n")
    }
}
