//! Output formatting for function command results.

use crate::output::Outputable;
use super::execute::FunctionResult;

impl Outputable for FunctionResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Function: {}.{}", self.module_pattern, self.function_pattern);
        lines.push(header);
        lines.push(String::new());

        if !self.functions.is_empty() {
            lines.push(format!("Found {} signature(s):", self.functions.len()));
            for func in &self.functions {
                let signature = format!(
                    "{}.{}/{}",
                    func.module, func.name, func.arity
                );
                lines.push(format!("  {}", signature));
                if !func.args.is_empty() {
                    lines.push(format!("       args: {}", func.args));
                }
                if !func.return_type.is_empty() {
                    lines.push(format!("       returns: {}", func.return_type));
                }
            }
        } else {
            lines.push("No functions found.".to_string());
        }

        lines.join("\n")
    }
}
