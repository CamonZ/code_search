//! Output formatting for calls-to command results.

use crate::output::Outputable;
use super::execute::CallsToResult;

impl Outputable for CallsToResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = if self.function_pattern.is_empty() {
            format!("Calls to: {}", self.module_pattern)
        } else {
            format!("Calls to: {}.{}", self.module_pattern, self.function_pattern)
        };
        lines.push(header);
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No callers found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} caller(s):", self.total_calls));
        lines.push(String::new());

        for module in &self.modules {
            lines.push(module.name.clone());

            for func in &module.functions {
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
