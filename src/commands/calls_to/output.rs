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

                for caller in &func.callers {
                    let kind_str = if caller.kind.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", caller.kind)
                    };
                    // Show caller module only if different from callee module
                    let caller_name = if caller.module == module.name {
                        format!("{}/{}", caller.function, caller.arity)
                    } else {
                        format!("{}.{}/{}", caller.module, caller.function, caller.arity)
                    };
                    lines.push(format!(
                        "    ← {}{} ({}:{}:{}) (L{})",
                        caller_name, kind_str, caller.file, caller.start_line, caller.end_line, caller.line
                    ));
                }
            }
        }

        lines.join("\n")
    }
}
