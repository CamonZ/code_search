//! Output formatting for unused command results.

use crate::output::Outputable;
use super::execute::UnusedResult;

impl Outputable for UnusedResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let mut filters = Vec::new();
        if let Some(pattern) = &self.module_filter {
            filters.push(format!("module: {}", pattern));
        }
        if self.private_only {
            filters.push("private only".to_string());
        }
        if self.public_only {
            filters.push("public only".to_string());
        }
        if self.exclude_generated {
            filters.push("excluding generated".to_string());
        }

        let filter_info = if filters.is_empty() {
            String::new()
        } else {
            format!(" ({})", filters.join(", "))
        };

        lines.push(format!("Unused functions in project '{}'{}", self.project, filter_info));
        lines.push(String::new());

        if !self.modules.is_empty() {
            lines.push(format!(
                "Found {} unused function(s) in {} module(s):",
                self.total_unused,
                self.modules.len()
            ));
            lines.push(String::new());

            for module in &self.modules {
                lines.push(format!("{} ({}):", module.name, module.file));
                for func in &module.functions {
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
