//! Output formatting for calls-from command results.

use crate::output::Outputable;
use super::execute::CallsFromResult;

impl Outputable for CallsFromResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = if self.function_pattern.is_empty() {
            format!("Calls from: {}", self.module_pattern)
        } else {
            format!("Calls from: {}.{}", self.module_pattern, self.function_pattern)
        };
        lines.push(header);
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No calls found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} call(s):", self.total_calls));
        lines.push(String::new());

        for module in &self.modules {
            lines.push(format!("{} ({})", module.name, module.file));

            for func in &module.functions {
                let kind_str = if func.kind.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", func.kind)
                };
                lines.push(format!(
                    "  {}/{} ({}:{}){}",
                    func.name, func.arity, func.start_line, func.end_line, kind_str
                ));

                for call in &func.calls {
                    // Format callee - show module only if different from caller
                    let callee = if call.module == module.name {
                        format!("{}/{}", call.function, call.arity)
                    } else {
                        format!("{}.{}/{}", call.module, call.function, call.arity)
                    };
                    lines.push(format!("    → {} (L{})", callee, call.line));
                }
            }
        }

        lines.join("\n")
    }
}
