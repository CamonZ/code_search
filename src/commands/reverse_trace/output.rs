//! Output formatting for reverse-trace command results.

use crate::output::Outputable;
use super::execute::ReverseTraceResult;

impl Outputable for ReverseTraceResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Reverse trace to: {}.{}", self.target_module, self.target_function);
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if !self.steps.is_empty() {
            lines.push(format!("Found {} caller(s) in chain:", self.steps.len()));
            for step in &self.steps {
                let indent = "  ".repeat(step.depth as usize);
                let caller = format!("{}.{}", step.caller_module, step.caller_function);
                let callee = format!("{}.{}/{}", step.callee_module, step.callee_function, step.callee_arity);
                lines.push(format!(
                    "{}[{}] {} ({}:{}) -> {}",
                    indent, step.depth, caller, step.file, step.line, callee
                ));
            }
        } else {
            lines.push("No callers found.".to_string());
        }

        lines.join("\n")
    }
}
