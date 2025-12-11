//! Output formatting for calls-to command results.

use crate::output::Outputable;
use crate::types::ModuleGroupResult;
use super::execute::CalleeFunction;

impl Outputable for ModuleGroupResult<CalleeFunction> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = if self.function_pattern.is_none() || self.function_pattern.as_ref().unwrap().is_empty() {
            format!("Calls to: {}", self.module_pattern)
        } else {
            format!("Calls to: {}.{}", self.module_pattern, self.function_pattern.as_ref().unwrap())
        };
        lines.push(header);
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push("No callers found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} caller(s):", self.total_items));
        lines.push(String::new());

        for module in &self.items {
            lines.push(module.name.clone());

            for func in &module.entries {
                lines.push(format!("  {}/{}", func.name, func.arity));

                for call in &func.callers {
                    // Use empty context file since callers come from different files
                    let formatted = call.format_incoming(&module.name, "");
                    lines.push(format!("    {}", formatted));
                }
            }
        }

        lines.join("\n")
    }
}
