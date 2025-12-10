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

/// Format root node (the starting function)
fn format_node(lines: &mut Vec<String>, node: &TraceNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind_str = if node.kind.is_empty() {
        String::new()
    } else {
        format!(" [{}]", node.kind)
    };

    // Extract just the filename from path
    let filename = node.file.rsplit('/').next().unwrap_or(&node.file);

    lines.push(format!(
        "{}{}.{}/{}{} ({}:L{}:{})",
        indent, node.module, node.function, node.arity, kind_str,
        filename, node.start_line, node.end_line
    ));

    for call in &node.calls {
        format_call(lines, call, depth + 1, &node.module, &node.file);
    }
}

fn format_call(lines: &mut Vec<String>, call: &TraceCall, depth: usize, parent_module: &str, parent_file: &str) {
    let indent = "  ".repeat(depth);

    // Show module only if different from parent
    let callee = if call.module == parent_module {
        format!("{}/{}", call.function, call.arity)
    } else {
        format!("{}.{}/{}", call.module, call.function, call.arity)
    };

    // Get callee definition info from children (if any)
    if let Some(child) = call.children.first() {
        let kind_str = if child.kind.is_empty() {
            String::new()
        } else {
            format!(" [{}]", child.kind)
        };

        // Extract just the filename
        let child_filename = child.file.rsplit('/').next().unwrap_or(&child.file);
        let parent_filename = parent_file.rsplit('/').next().unwrap_or(parent_file);

        let location = if child_filename == parent_filename {
            format!("L{}:{}", child.start_line, child.end_line)
        } else {
            format!("{}:L{}:{}", child_filename, child.start_line, child.end_line)
        };

        lines.push(format!(
            "{}→ @ L{} {}{} ({})",
            indent, call.line, callee, kind_str, location
        ));

        // Recurse into child's calls
        for sub_call in &child.calls {
            format_call(lines, sub_call, depth + 1, &child.module, &child.file);
        }
    } else {
        // No definition info available (leaf node)
        lines.push(format!("{}→ @ L{} {}", indent, call.line, callee));
    }
}
