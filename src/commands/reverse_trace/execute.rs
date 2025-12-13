use std::collections::HashMap;
use std::error::Error;

use super::ReverseTraceCmd;
use crate::commands::Execute;
use crate::queries::reverse_trace::{reverse_trace_calls, ReverseTraceStep};
use crate::types::{TraceDirection, TraceEntry, TraceResult};

impl TraceResult {
    /// Build a flattened reverse-trace from ReverseTraceStep objects
    pub fn from_steps(
        target_module: String,
        target_function: String,
        max_depth: u32,
        steps: Vec<ReverseTraceStep>,
    ) -> Self {
        let mut entries = Vec::new();
        let mut entry_index_map: HashMap<(String, String, i64, i64), usize> = HashMap::new();

        if steps.is_empty() {
            return Self::empty(target_module, target_function, max_depth, TraceDirection::Backward);
        }

        // Group steps by depth
        let mut by_depth: HashMap<i64, Vec<&ReverseTraceStep>> = HashMap::new();
        for step in &steps {
            by_depth.entry(step.depth).or_default().push(step);
        }

        // Process depth 1 (direct callers of target function)
        if let Some(depth1_steps) = by_depth.get(&1) {
            let mut filter = crate::dedup::DeduplicationFilter::new();

            for step in depth1_steps {
                let caller_key = (
                    step.caller_module.clone(),
                    step.caller_function.clone(),
                    step.caller_arity,
                    1i64,
                );

                // Add caller as root entry if not already added
                if filter.should_process(caller_key.clone()) {
                    let entry_idx = entries.len();
                    entries.push(TraceEntry {
                        module: step.caller_module.clone(),
                        function: step.caller_function.clone(),
                        arity: step.caller_arity,
                        kind: step.caller_kind.clone(),
                        start_line: step.caller_start_line,
                        end_line: step.caller_end_line,
                        file: step.file.clone(),
                        depth: 1,
                        line: step.line,
                        parent_index: None,
                    });
                    entry_index_map.insert(caller_key, entry_idx);
                }
            }
        }

        // Process deeper levels (additional callers)
        for depth in 2..=max_depth as i64 {
            if let Some(depth_steps) = by_depth.get(&depth) {
                let mut filter = crate::dedup::DeduplicationFilter::new();

                for step in depth_steps {
                    let caller_key = (
                        step.caller_module.clone(),
                        step.caller_function.clone(),
                        step.caller_arity,
                        depth,
                    );

                    // Find parent index (the callee at previous depth, which is what called this caller)
                    let parent_key = (
                        step.callee_module.clone(),
                        step.callee_function.clone(),
                        step.callee_arity,
                        depth - 1,
                    );

                    let parent_index = entry_index_map.get(&parent_key).copied();

                    if filter.should_process(caller_key.clone()) && parent_index.is_some() {
                        let entry_idx = entries.len();
                        entries.push(TraceEntry {
                            module: step.caller_module.clone(),
                            function: step.caller_function.clone(),
                            arity: step.caller_arity,
                            kind: step.caller_kind.clone(),
                            start_line: step.caller_start_line,
                            end_line: step.caller_end_line,
                            file: step.file.clone(),
                            depth,
                            line: step.line,
                            parent_index,
                        });
                        entry_index_map.insert(caller_key, entry_idx);
                    }
                }
            }
        }

        let total_items = entries.len();

        Self {
            module: target_module,
            function: target_function,
            max_depth,
            direction: TraceDirection::Backward,
            total_items,
            entries,
        }
    }
}

impl Execute for ReverseTraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
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

        Ok(TraceResult::from_steps(
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
        let result = TraceResult::from_steps(
            "TestModule".to_string(),
            "test_func".to_string(),
            5,
            vec![],
        );
        assert_eq!(result.total_items, 0);
        assert_eq!(result.entries.len(), 0);
    }
}
