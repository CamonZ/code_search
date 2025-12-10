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
                for caller in &func.callers {
                    let kind_str = if caller.kind.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", caller.kind)
                    };
                    lines.push(format!(
                        "    ← {}.{}/{} ({}:{}:{} L{}){}",
                        caller.module, caller.function, caller.arity,
                        caller.file, caller.start_line, caller.end_line,
                        caller.line, kind_str
                    ));
                }
            }
        }

        lines.join("\n")
    }
}
