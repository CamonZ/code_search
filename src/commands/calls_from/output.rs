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

        if !self.calls.is_empty() {
            lines.push(format!("Found {} call(s):", self.calls.len()));
            for call in &self.calls {
                let caller = format!("{}.{}", call.caller_module, call.caller_function);
                let callee = format!("{}.{}/{}", call.callee_module, call.callee_function, call.callee_arity);
                lines.push(format!(
                    "  {} ({}:{}) -> {}",
                    caller, call.file, call.line, callee
                ));
            }
        } else {
            lines.push("No calls found.".to_string());
        }

        lines.join("\n")
    }
}
