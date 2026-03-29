//! Unified types for trace and reverse-trace commands.

use serde::Serialize;

/// Direction of trace traversal
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceDirection {
    #[default]
    Forward,
    Backward,
}

/// A single entry in the trace tree (flattened representation)
#[derive(Debug, Clone, Serialize)]
pub struct TraceEntry {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub depth: i64,
    pub line: i64,                     // Line where the call happens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_index: Option<usize>,   // Index in entries list of parent
}

/// Result of trace or reverse-trace command execution
#[derive(Debug, Default, Serialize)]
pub struct TraceResult {
    pub module: String,
    pub function: String,
    pub max_depth: u32,
    pub direction: TraceDirection,
    pub total_items: usize,            // total_calls or total_callers
    pub entries: Vec<TraceEntry>,
}

impl TraceResult {
    /// Create an empty trace result
    pub fn empty(module: String, function: String, max_depth: u32, direction: TraceDirection) -> Self {
        Self {
            module,
            function,
            max_depth,
            direction,
            total_items: 0,
            entries: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stores_module_and_function() {
        let result = TraceResult::empty(
            "MyModule".to_string(),
            "my_func".to_string(),
            5,
            TraceDirection::Backward,
        );

        assert_eq!(result.module, "MyModule");
        assert_eq!(result.function, "my_func");
        assert_eq!(result.max_depth, 5);
        assert!(matches!(result.direction, TraceDirection::Backward));
        assert_eq!(result.total_items, 0);
        assert!(result.entries.is_empty());
    }
}
