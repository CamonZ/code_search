//! Output formatting for depends-on command results.

use crate::output::Outputable;
use super::execute::DependsOnResult;

impl Outputable for DependsOnResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Dependencies of: {}", self.source_module));
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No dependencies found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} call(s) to {} module(s):", self.total_calls, self.modules.len()));
        lines.push(String::new());

        for module in &self.modules {
            lines.push(format!("{}:", module.name));
            for func in &module.functions {
                lines.push(format!("  {}/{}:", func.name, func.arity));
                for call in &func.callers {
                    // Use empty context since callers come from different files
                    let formatted = call.format_incoming(&module.name, "");
                    lines.push(format!("    {}", formatted));
                }
            }
        }

        lines.join("\n")
    }
}
