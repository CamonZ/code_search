//! Output formatting for depends-on command results.

use crate::output::Outputable;
use super::execute::DependsOnResult;

impl Outputable for DependsOnResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Dependencies of: {}", self.source_module));
        lines.push(String::new());

        if !self.dependencies.is_empty() {
            lines.push(format!("Found {} module(s):", self.dependencies.len()));
            for dep in &self.dependencies {
                lines.push(format!("  {} ({} calls)", dep.module, dep.call_count));
            }
        } else {
            lines.push("No dependencies found.".to_string());
        }

        lines.join("\n")
    }
}
