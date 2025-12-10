//! Output formatting for reverse-trace command results.

use crate::output::Outputable;
use super::execute::{ReverseTraceNode, ReverseTraceResult, ReverseTraceTarget};

impl Outputable for ReverseTraceResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Reverse trace to: {}.{}", self.target_module, self.target_function);
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if self.roots.is_empty() {
            lines.push("No callers found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} caller(s) in chain:", self.total_callers));
        lines.push(String::new());

        for root in &self.roots {
            format_node(&mut lines, root, 0);
        }

        lines.join("\n")
    }
}

fn format_node(lines: &mut Vec<String>, node: &ReverseTraceNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind_str = if node.kind.is_empty() {
        String::new()
    } else {
        format!(" [{}]", node.kind)
    };

    lines.push(format!(
        "{}{}.{}/{} ({}:{}:{}){}",
        indent, node.module, node.function, node.arity,
        node.file, node.start_line, node.end_line, kind_str
    ));

    for target in &node.targets {
        format_target(lines, target, depth + 1, &node.module);
    }

    // Recursively format callers (going upward in the call chain)
    for caller in &node.callers {
        format_node(lines, caller, depth + 1);
    }
}

fn format_target(lines: &mut Vec<String>, target: &ReverseTraceTarget, depth: usize, caller_module: &str) {
    let indent = "  ".repeat(depth);

    // Show module only if different from caller
    let callee = if target.module == caller_module {
        format!("{}/{}", target.function, target.arity)
    } else {
        format!("{}.{}/{}", target.module, target.function, target.arity)
    };

    lines.push(format!("{}→ {} (L{})", indent, callee, target.line));
}
