use std::collections::HashMap;
use std::error::Error;

use super::TraceCmd;
use crate::commands::Execute;
use crate::queries::trace::trace_calls;
use crate::types::{Call, TraceDirection, TraceEntry, TraceResult};

impl TraceResult {
    /// Build a flattened trace from Call objects
    pub fn from_calls(
        start_module: String,
        start_function: String,
        max_depth: u32,
        calls: Vec<Call>,
    ) -> Self {
        let mut entries = Vec::new();
        let mut entry_index_map: HashMap<(String, String, i64, i64), usize> = HashMap::new();

        // Add the starting function as the root entry at depth 0
        entries.push(TraceEntry {
            module: start_module.clone(),
            function: start_function.clone(),
            arity: 0, // Will be updated from first call if available
            kind: String::new(),
            start_line: 0,
            end_line: 0,
            file: String::new(),
            depth: 0,
            line: 0,
            parent_index: None,
        });
        entry_index_map.insert((start_module.clone(), start_function.clone(), 0, 0), 0);

        if calls.is_empty() {
            return Self::empty(start_module, start_function, max_depth, TraceDirection::Forward);
        }

        // Group calls by depth
        let mut by_depth: HashMap<i64, Vec<&Call>> = HashMap::new();
        for call in &calls {
            if let Some(depth) = call.depth {
                by_depth.entry(depth).or_default().push(call);
            }
        }

        // Process depth 1 (direct callees from start function)
        if let Some(depth1_calls) = by_depth.get(&1) {
            let mut processed = std::collections::HashSet::new();

            for call in depth1_calls {
                let callee_key = (
                    call.callee.module.clone(),
                    call.callee.name.clone(),
                    call.callee.arity,
                    1i64,
                );

                // Add callee as child entry if not already added
                if !processed.contains(&callee_key) {
                    let entry_idx = entries.len();
                    entries.push(TraceEntry {
                        module: call.callee.module.clone(),
                        function: call.callee.name.clone(),
                        arity: call.callee.arity,
                        kind: call.callee.kind.clone().unwrap_or_default(),
                        start_line: call.callee.start_line.unwrap_or(0),
                        end_line: call.callee.end_line.unwrap_or(0),
                        file: call.callee.file.clone().unwrap_or_default(),
                        depth: 1,
                        line: call.line,
                        parent_index: Some(0), // Parent is the starting function at index 0
                    });
                    entry_index_map.insert(callee_key.clone(), entry_idx);
                    processed.insert(callee_key);
                }
            }
        }

        // Process deeper levels
        for depth in 2..=max_depth as i64 {
            if let Some(depth_calls) = by_depth.get(&depth) {
                let mut processed = std::collections::HashSet::new();

                for call in depth_calls {
                    let callee_key = (
                        call.callee.module.clone(),
                        call.callee.name.clone(),
                        call.callee.arity,
                        depth,
                    );

                    // Find parent index (the caller at previous depth)
                    let caller_key = (
                        call.caller.module.clone(),
                        call.caller.name.clone(),
                        call.caller.arity,
                        depth - 1,
                    );

                    // Find the parent entry
                    let parent_index = entry_index_map.get(&caller_key).copied();

                    if !processed.contains(&callee_key) && parent_index.is_some() {
                        let entry_idx = entries.len();
                        entries.push(TraceEntry {
                            module: call.callee.module.clone(),
                            function: call.callee.name.clone(),
                            arity: call.callee.arity,
                            kind: call.callee.kind.clone().unwrap_or_default(),
                            start_line: call.callee.start_line.unwrap_or(0),
                            end_line: call.callee.end_line.unwrap_or(0),
                            file: call.callee.file.clone().unwrap_or_default(),
                            depth,
                            line: call.line,
                            parent_index,
                        });
                        entry_index_map.insert(callee_key.clone(), entry_idx);
                        processed.insert(callee_key);
                    }
                }
            }
        }

        let total_items = entries.len() - 1; // Exclude the root entry from count

        Self {
            module: start_module,
            function: start_function,
            max_depth,
            direction: TraceDirection::Forward,
            total_items,
            entries,
        }
    }
}

impl Execute for TraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.depth,
            self.limit,
        )?;

        Ok(TraceResult::from_calls(
            self.module,
            self.function,
            self.depth,
            calls,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_trace() {
        let result = TraceResult::from_calls("TestModule".to_string(), "test_func".to_string(), 5, vec![]);
        assert_eq!(result.total_items, 0);
        assert_eq!(result.entries.len(), 0);
    }
}
