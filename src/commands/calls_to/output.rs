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

        if !self.calls.is_empty() {
            lines.push(format!("Found {} caller(s):", self.calls.len()));
            for call in &self.calls {
                let kind_str = if call.caller_kind.is_empty() {
                    String::new()
                } else {
                    format!("[{}] ", call.caller_kind)
                };
                let caller = format!("{}.{}", call.caller_module, call.caller_function);
                let callee = format!("{}.{}/{}", call.callee_module, call.callee_function, call.callee_arity);
                lines.push(format!(
                    "  {}{} ({}:{}) -> {}",
                    kind_str, caller, call.file, call.line, callee
                ));
            }
        } else {
            lines.push("No callers found.".to_string());
        }

        lines.join("\n")
    }
}
