//! Output formatting for unused command results.

use crate::output::Outputable;
use db::types::ModuleCollectionResult;
use super::execute::UnusedFunc;

impl Outputable for ModuleCollectionResult<UnusedFunc> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let filter_info = if self.module_pattern != "*" {
            format!(" (module: {})", self.module_pattern)
        } else {
            String::new()
        };

        lines.push(format!("Unused functions{}", filter_info));
        lines.push(String::new());

        if !self.items.is_empty() {
            lines.push(format!(
                "Found {} unused function(s) in {} module(s):",
                self.total_items,
                self.items.len()
            ));
            lines.push(String::new());

            for module in &self.items {
                lines.push(format!("{} ({}):", module.name, module.file));
                for func in &module.entries {
                    lines.push(format!(
                        "  {}/{} [{}] L{}",
                        func.name, func.arity, func.kind, func.line
                    ));
                }
            }
        } else {
            lines.push("No unused functions found.".to_string());
        }

        lines.join("\n")
    }
}
