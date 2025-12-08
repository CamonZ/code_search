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
                let sig = format!("{}.{}/{}", loc.module, loc.name, loc.arity);
                lines.push(format!("  {} ({})", sig, loc.kind));
                lines.push(format!("       {}", loc.format_location()));
            }
        } else {
            lines.push("No locations found.".to_string());
        }

        lines.join("\n")
    }
}
