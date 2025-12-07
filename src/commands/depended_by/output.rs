//! Output formatting for depended-by command results.

use crate::output::Outputable;
use super::execute::DependedByResult;

impl Outputable for DependedByResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Modules that depend on: {}", self.target_module));
        lines.push(String::new());

        if !self.dependents.is_empty() {
            lines.push(format!("Found {} module(s):", self.dependents.len()));
            for dep in &self.dependents {
                lines.push(format!("  {} ({} calls)", dep.module, dep.call_count));
            }
        } else {
            lines.push("No dependents found.".to_string());
        }

        lines.join("\n")
    }
}
