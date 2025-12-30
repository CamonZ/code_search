use std::collections::HashMap;
use std::error::Error;

use super::ReverseTraceCmd;
use crate::commands::Execute;
use db::queries::reverse_trace::{reverse_trace_calls, ReverseTraceStep};
use db::types::{TraceDirection, TraceEntry, TraceResult};

/// Build a flattened reverse-trace from ReverseTraceStep objects
fn build_reverse_trace_result(
    target_module: String,
    target_function: String,
    max_depth: u32,
    steps: Vec<ReverseTraceStep>,
) -> TraceResult {
    let mut entries = Vec::new();
    let mut entry_index_map: HashMap<(String, String, i64, i64), usize> = HashMap::new();

    if steps.is_empty() {
        return TraceResult::empty(target_module, target_function, max_depth, TraceDirection::Backward);
    }

    // Group steps by depth
    let mut by_depth: HashMap<i64, Vec<&ReverseTraceStep>> = HashMap::new();
    for step in &steps {
        by_depth.entry(step.depth).or_default().push(step);
    }

    // Process depth 1 (direct callers of target function)
    if let Some(depth1_steps) = by_depth.get(&1) {
        for step in depth1_steps {
            let caller_key = (
                step.caller_module.clone(),
                step.caller_function.clone(),
                step.caller_arity,
                1i64,
            );

            // Add caller as root entry if not already added (use HashMap for dedup check)
            if !entry_index_map.contains_key(&caller_key) {
                let entry_idx = entries.len();
                // Insert into HashMap before pushing (reuse caller_key)
                entry_index_map.insert(caller_key.clone(), entry_idx);

                entries.push(TraceEntry {
                    module: caller_key.0,
                    function: caller_key.1,
                    arity: caller_key.2,
                    kind: step.caller_kind.clone(),
                    start_line: step.caller_start_line,
                    end_line: step.caller_end_line,
                    file: step.file.clone(),
                    depth: 1,
                    line: step.line,
                    parent_index: None,
                });
            }
        }
    }

    // Process deeper levels (additional callers)
    for depth in 2..=max_depth as i64 {
        if let Some(depth_steps) = by_depth.get(&depth) {
            for step in depth_steps {
                let caller_key = (
                    step.caller_module.clone(),
                    step.caller_function.clone(),
                    step.caller_arity,
                    depth,
                );

                // Check if we already have this caller at this depth using HashMap
                if !entry_index_map.contains_key(&caller_key) {
                    // Find parent index using HashMap (O(1) lookup)
                    let parent_key = (
                        step.callee_module.clone(),
                        step.callee_function.clone(),
                        step.callee_arity,
                        depth - 1,
                    );
                    let parent_index = entry_index_map.get(&parent_key).copied();

                    if parent_index.is_some() {
                        let entry_idx = entries.len();
                        // Insert into HashMap before pushing (reuse caller_key)
                        entry_index_map.insert(caller_key.clone(), entry_idx);

                        entries.push(TraceEntry {
                            module: caller_key.0,
                            function: caller_key.1,
                            arity: caller_key.2,
                            kind: step.caller_kind.clone(),
                            start_line: step.caller_start_line,
                            end_line: step.caller_end_line,
                            file: step.file.clone(),
                            depth,
                            line: step.line,
                            parent_index,
                        });
                    }
                }
            }
        }
    }

    let total_items = entries.len();

    TraceResult {
        module: target_module,
        function: target_function,
        max_depth,
        direction: TraceDirection::Backward,
        total_items,
        entries,
    }
}

impl Execute for ReverseTraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let steps = reverse_trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.common.project,
            self.common.regex,
            self.depth,
            self.common.limit,
        )?;

        Ok(build_reverse_trace_result(
            self.module,
            self.function,
            self.depth,
            steps,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_reverse_trace() {
        let result = build_reverse_trace_result(
            "TestModule".to_string(),
            "test_func".to_string(),
            5,
            vec![],
        );
        assert_eq!(result.total_items, 0);
        assert_eq!(result.entries.len(), 0);
    }
}
