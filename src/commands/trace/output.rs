//! Output formatting for trace and reverse-trace command results.

use crate::output::Outputable;
use crate::types::{TraceResult, TraceDirection};

impl Outputable for TraceResult {
    fn to_table(&self) -> String {
        match self.direction {
            TraceDirection::Forward => format_trace(self),
            TraceDirection::Backward => format_reverse_trace(self),
        }
    }
}

/// Format a forward trace
fn format_trace(result: &TraceResult) -> String {
    let mut lines = Vec::new();

    let header = format!("Trace from: {}.{}", result.module, result.function);
    lines.push(header);
    lines.push(format!("Max depth: {}", result.max_depth));
    lines.push(String::new());

    if result.entries.is_empty() {
        lines.push("No calls found.".to_string());
        return lines.join("\n");
    }

    lines.push(format!("Found {} call(s) in chain:", result.total_items));
    lines.push(String::new());

    // Find root entries (those with no parent)
    for (idx, entry) in result.entries.iter().enumerate() {
        if entry.parent_index.is_none() {
            format_entry(&mut lines, &result.entries, idx, 0);
        }
    }

    lines.join("\n")
}

/// Format a reverse trace
fn format_reverse_trace(result: &TraceResult) -> String {
    let mut lines = Vec::new();

    let header = format!("Reverse trace to: {}.{}", result.module, result.function);
    lines.push(header);
    lines.push(format!("Max depth: {}", result.max_depth));
    lines.push(String::new());

    if result.entries.is_empty() {
        lines.push("No callers found.".to_string());
        return lines.join("\n");
    }

    lines.push(format!("Found {} caller(s) in chain:", result.total_items));
    lines.push(String::new());

    // Find root entries (those with no parent)
    for (idx, entry) in result.entries.iter().enumerate() {
        if entry.parent_index.is_none() {
            format_reverse_entry(&mut lines, &result.entries, idx, 0);
        }
    }

    lines.join("\n")
}

/// Format a reverse trace entry (callers going up the chain)
fn format_reverse_entry(lines: &mut Vec<String>, entries: &[crate::types::TraceEntry], idx: usize, depth: usize) {
    let entry = &entries[idx];
    let indent = "  ".repeat(depth);
    let kind_str = if entry.kind.is_empty() {
        String::new()
    } else {
        format!(" [{}]", entry.kind)
    };

    // Extract just the filename from path
    let filename = entry.file.rsplit('/').next().unwrap_or(&entry.file);

    // For root entries (no parent), show without prefix
    if entry.parent_index.is_none() {
        lines.push(format!(
            "{}{}.{}/{}{} ({}:L{}:{})",
            indent, entry.module, entry.function, entry.arity, kind_str,
            filename, entry.start_line, entry.end_line
        ));
    } else {
        // For child entries, show with arrow indicating "called by" relationship
        lines.push(format!(
            "{}← @ L{} {}.{}/{}{} ({}:L{}:{})",
            indent, entry.line, entry.module, entry.function, entry.arity, kind_str,
            filename, entry.start_line, entry.end_line
        ));
    }

    // Find children (additional callers going up the chain)
    for (child_idx, child) in entries.iter().enumerate() {
        if child.parent_index == Some(idx) {
            format_reverse_entry(lines, entries, child_idx, depth + 1);
        }
    }
}

/// Recursively format an entry and its children
fn format_entry(lines: &mut Vec<String>, entries: &[crate::types::TraceEntry], idx: usize, depth: usize) {
    let entry = &entries[idx];
    let indent = "  ".repeat(depth);
    let kind_str = if entry.kind.is_empty() {
        String::new()
    } else {
        format!(" [{}]", entry.kind)
    };

    // Extract just the filename from path
    let filename = entry.file.rsplit('/').next().unwrap_or(&entry.file);

    lines.push(format!(
        "{}{}.{}/{}{} ({}:L{}:{})",
        indent, entry.module, entry.function, entry.arity, kind_str,
        filename, entry.start_line, entry.end_line
    ));

    // Find children of this entry
    for (child_idx, child) in entries.iter().enumerate() {
        if child.parent_index == Some(idx) {
            format_call(lines, entries, child_idx, depth + 1, &entry.module, &entry.file);
        }
    }
}

/// Format a child call/caller entry
fn format_call(
    lines: &mut Vec<String>,
    entries: &[crate::types::TraceEntry],
    idx: usize,
    depth: usize,
    parent_module: &str,
    parent_file: &str,
) {
    let entry = &entries[idx];
    let indent = "  ".repeat(depth);

    // Show module only if different from parent
    let name = if entry.module == parent_module {
        format!("{}/{}", entry.function, entry.arity)
    } else {
        format!("{}.{}/{}", entry.module, entry.function, entry.arity)
    };

    let kind_str = if entry.kind.is_empty() {
        String::new()
    } else {
        format!(" [{}]", entry.kind)
    };

    // Extract just the filename
    let child_filename = entry.file.rsplit('/').next().unwrap_or(&entry.file);
    let parent_filename = parent_file.rsplit('/').next().unwrap_or(parent_file);

    let location = if child_filename == parent_filename {
        format!("L{}:{}", entry.start_line, entry.end_line)
    } else {
        format!("{}:L{}:{}", child_filename, entry.start_line, entry.end_line)
    };

    lines.push(format!(
        "{}→ @ L{} {}{} ({})",
        indent, entry.line, name, kind_str, location
    ));

    // Recurse into children of this entry
    for (child_idx, child) in entries.iter().enumerate() {
        if child.parent_index == Some(idx) {
            format_call(lines, entries, child_idx, depth + 1, &entry.module, &entry.file);
        }
    }
}
