//! Output formatting for function command results.

use crate::output::Outputable;
use crate::types::ModuleGroupResult;
use super::execute::FuncSig;

impl Outputable for ModuleGroupResult<FuncSig> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let function_pattern = self.function_pattern.as_ref().map(|s| s.as_str()).unwrap_or("*");
        let header = format!("Function: {}.{}", self.module_pattern, function_pattern);
        lines.push(header);
        lines.push(String::new());

        if !self.items.is_empty() {
            lines.push(format!(
                "Found {} signature(s) in {} module(s):",
                self.total_items,
                self.items.len()
            ));
            lines.push(String::new());

            for module in &self.items {
                lines.push(format!("{}:", module.name));
                for func in &module.entries {
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
