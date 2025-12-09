//! Output formatting for location command results.

use crate::output::Outputable;
use super::execute::LocationResult;

impl Outputable for LocationResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Location: {}.{}", self.module_pattern, self.function_pattern));
        lines.push(String::new());

        if !self.locations.is_empty() {
            lines.push(format!("Found {} location(s):", self.locations.len()));
            for loc in &self.locations {
                // Build signature with pattern if available
                let sig = if loc.pattern.is_empty() {
                    format!("{}.{}/{}", loc.module, loc.name, loc.arity)
                } else {
                    format!("{}.{}({})", loc.module, loc.name, loc.pattern)
                };

                // Add guard if present
                let guard_str = if loc.guard.is_empty() {
                    String::new()
                } else {
                    format!(" when {}", loc.guard)
                };

                lines.push(format!("  {} ({}){}", sig, loc.kind, guard_str));
                lines.push(format!("       {}", loc.format_location()));
            }
        } else {
            lines.push("No locations found.".to_string());
        }

        lines.join("\n")
    }
}
