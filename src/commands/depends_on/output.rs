//! Output formatting for depends-on command results.

use crate::output::Outputable;
use crate::types::ModuleGroupResult;
use super::execute::DependencyFunction;

impl Outputable for ModuleGroupResult<DependencyFunction> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Dependencies of: {}", self.module_pattern));
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push("No dependencies found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} call(s) to {} module(s):", self.total_items, self.items.len()));
        lines.push(String::new());

        for module in &self.items {
            lines.push(format!("{}:", module.name));
            for func in &module.entries {
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
