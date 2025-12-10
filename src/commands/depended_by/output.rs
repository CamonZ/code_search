//! Output formatting for depended-by command results.

use crate::output::Outputable;
use super::execute::DependedByResult;

impl Outputable for DependedByResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Modules that depend on: {}", self.target_module));
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No dependents found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} call(s) from {} module(s):", self.total_calls, self.modules.len()));
        lines.push(String::new());

        for module in &self.modules {
            lines.push(format!("{}:", module.name));
            for caller in &module.callers {
                let kind_str = if caller.kind.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", caller.kind)
                };
                lines.push(format!(
                    "  {}/{} ({}:{}:{}){}:",
                    caller.function, caller.arity,
                    caller.file, caller.start_line, caller.end_line, kind_str
                ));
                for target in &caller.targets {
                    lines.push(format!("    → {}/{} (L{})", target.function, target.arity, target.line));
                }
            }
        }

        lines.join("\n")
    }
}
