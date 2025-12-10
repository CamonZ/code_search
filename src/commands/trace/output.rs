//! Output formatting for trace command results.

use crate::output::Outputable;
use super::execute::{TraceCall, TraceNode, TraceResult};

impl Outputable for TraceResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Trace from: {}.{}", self.start_module, self.start_function);
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if self.roots.is_empty() {
            lines.push("No calls found.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Found {} call(s) in chain:", self.total_calls));
        lines.push(String::new());

        for root in &self.roots {
            format_node(&mut lines, root, 0);
        }

        lines.join("\n")
    }
}

fn format_node(lines: &mut Vec<String>, node: &TraceNode, depth: usize) {
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

    for call in &node.calls {
        format_call(lines, call, depth + 1, &node.module);
    }
}

fn format_call(lines: &mut Vec<String>, call: &TraceCall, depth: usize, parent_module: &str) {
    let indent = "  ".repeat(depth);

    // Show module only if different from parent
    let callee = if call.module == parent_module {
        format!("{}/{}", call.function, call.arity)
    } else {
        format!("{}.{}/{}", call.module, call.function, call.arity)
    };

    lines.push(format!("{}→ {} (L{})", indent, callee, call.line));

    // Recursively format children
    for child in &call.children {
        format_node(lines, child, depth + 1);
    }
}
