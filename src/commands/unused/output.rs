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

        if !self.functions.is_empty() {
            lines.push(format!("Found {} unused function(s):", self.functions.len()));
            for func in &self.functions {
                let sig = format!("{}.{}/{}", func.module, func.name, func.arity);
                lines.push(format!("  [{}] {}", func.kind, sig));
                lines.push(format!("       {}:{}", func.file, func.line));
            }
        } else {
            lines.push("No unused functions found.".to_string());
        }

        lines.join("\n")
    }
}
