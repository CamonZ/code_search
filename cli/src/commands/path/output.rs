//! Output formatting for path command results.

use crate::output::Outputable;
use super::execute::PathResult;

impl Outputable for PathResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!(
            "Path from: {}.{} to: {}.{}",
            self.from_module, self.from_function, self.to_module, self.to_function
        );
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if !self.paths.is_empty() {
            lines.push(format!("Found {} path(s):", self.paths.len()));
            for (i, path) in self.paths.iter().enumerate() {
                lines.push(String::new());
                lines.push(format!("Path {}:", i + 1));
                for step in &path.steps {
                    let indent = "  ".repeat(step.depth as usize);
                    let caller = format!("{}.{}", step.caller_module, step.caller_function);
                    let callee = format!("{}.{}/{}", step.callee_module, step.callee_function, step.callee_arity);
                    lines.push(format!(
                        "{}[{}] {} ({}:{}) -> {}",
                        indent, step.depth, caller, step.file, step.line, callee
                    ));
                }
            }
        } else {
            lines.push("No path found.".to_string());
        }

        lines.join("\n")
    }
}
