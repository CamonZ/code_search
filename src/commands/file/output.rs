//! Output formatting for file command results.

use crate::output::Outputable;
use super::execute::FileResult;

impl Outputable for FileResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Functions in files matching '{}' (project: {})", self.file_pattern, self.project));
        lines.push(String::new());

        if !self.files.is_empty() {
            for file_info in &self.files {
                lines.push(format!("{}:", file_info.file));
                for func in &file_info.functions {
                    // Build signature with pattern if available
                    let sig = if func.pattern.is_empty() {
                        format!("{}.{}/{}", func.module, func.name, func.arity)
                    } else {
                        format!("{}.{}({})", func.module, func.name, func.pattern)
                    };

                    // Add guard if present
                    let guard_str = if func.guard.is_empty() {
                        String::new()
                    } else {
                        format!(" when {}", func.guard)
                    };

                    lines.push(format!(
                        "  {:>4}-{:<4} [{}] {}{}",
                        func.start_line, func.end_line, func.kind, sig, guard_str
                    ));
                }
                lines.push(String::new());
            }
            // Remove trailing empty line
            if lines.last() == Some(&String::new()) {
                lines.pop();
            }
        } else {
            lines.push("No files found.".to_string());
        }

        lines.join("\n")
    }
}
