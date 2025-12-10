//! Output formatting for function command results.

use crate::output::Outputable;
use super::execute::FunctionResult;

impl Outputable for FunctionResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Function: {}.{}", self.module_pattern, self.function_pattern);
        lines.push(header);
        lines.push(String::new());

        if !self.modules.is_empty() {
            lines.push(format!(
                "Found {} signature(s) in {} module(s):",
                self.total_functions,
                self.modules.len()
            ));
            lines.push(String::new());

            for module in &self.modules {
                lines.push(format!("{}:", module.name));
                for func in &module.functions {
                    lines.push(format!("  {}/{}", func.name, func.arity));
                    if !func.args.is_empty() {
                        lines.push(format!("    args: {}", func.args));
                    }
                    if !func.return_type.is_empty() {
                        lines.push(format!("    returns: {}", func.return_type));
                    }
                }
            }
        } else {
            lines.push("No functions found.".to_string());
        }

        lines.join("\n")
    }
}
