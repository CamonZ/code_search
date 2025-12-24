use std::collections::HashMap;
use std::error::Error;

use super::TraceCmd;
use crate::commands::Execute;
use db::queries::trace::trace_calls;
use db::types::{Call, TraceDirection, TraceEntry, TraceResult};

fn build_trace_result(
    start_module: String,
    start_function: String,
    max_depth: u32,
    calls: Vec<Call>,
) -> TraceResult {
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
        return TraceResult::empty(start_module, start_function, max_depth, TraceDirection::Forward);
    }

    // Group calls by depth, consuming the Vec to take ownership
    let mut by_depth: HashMap<i64, Vec<Call>> = HashMap::new();
    for call in calls {
        if let Some(depth) = call.depth {
            by_depth.entry(depth).or_default().push(call);
        }
    }

    // Process depth 1 (direct callees from start function)
    if let Some(depth1_calls) = by_depth.remove(&1) {
        // Track seen entries by index into entries vec (avoids storing strings)
        let mut seen_at_depth: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for call in depth1_calls {
            // Check if we already have this callee at this depth
            let existing = entries.iter().position(|e| {
                e.depth == 1
                    && e.module == call.callee.module.as_ref()
                    && e.function == call.callee.name.as_ref()
                    && e.arity == call.callee.arity
            });

            if (existing.is_none() || seen_at_depth.insert(existing.unwrap_or(usize::MAX)))
                && existing.is_none() {
                    let entry_idx = entries.len();
                    // Convert from Rc<str> to String for storage
                    let module = call.callee.module.to_string();
                    let function = call.callee.name.to_string();
                    let arity = call.callee.arity;
                    entry_index_map.insert((module.clone(), function.clone(), arity, 1i64), entry_idx);
                    entries.push(TraceEntry {
                        module,
                        function,
                        arity,
                        kind: call.callee.kind.as_deref().unwrap_or("").to_string(),
                        start_line: call.callee.start_line.unwrap_or(0),
                        end_line: call.callee.end_line.unwrap_or(0),
                        file: call.callee.file.as_deref().unwrap_or("").to_string(),
                        depth: 1,
                        line: call.line,
                        parent_index: Some(0),
                    });
                }
        }
    }

    // Process deeper levels
    for depth in 2..=max_depth as i64 {
        if let Some(depth_calls) = by_depth.remove(&depth) {
            for call in depth_calls {
                // Check if we already have this callee at this depth using HashMap
                let callee_key = (
                    call.callee.module.to_string(),
                    call.callee.name.to_string(),
                    call.callee.arity,
                    depth,
                );

                if !entry_index_map.contains_key(&callee_key) {
                    // Find parent index using HashMap (O(1) lookup)
                    let parent_key = (
                        call.caller.module.to_string(),
                        call.caller.name.to_string(),
                        call.caller.arity,
                        depth - 1,
                    );
                    let parent_index = entry_index_map.get(&parent_key).copied();

                    if parent_index.is_some() {
                        let entry_idx = entries.len();
                        // Insert into HashMap before pushing (reuse callee_key)
                        entry_index_map.insert(callee_key.clone(), entry_idx);

                        // Convert from Rc<str> to String for storage
                        entries.push(TraceEntry {
                            module: callee_key.0,
                            function: callee_key.1,
                            arity: callee_key.2,
                            kind: call.callee.kind.as_deref().unwrap_or("").to_string(),
                            start_line: call.callee.start_line.unwrap_or(0),
                            end_line: call.callee.end_line.unwrap_or(0),
                            file: call.callee.file.as_deref().unwrap_or("").to_string(),
                            depth,
                            line: call.line,
                            parent_index,
                        });
                    }
                }
            }
        }
    }

    let total_items = entries.len() - 1; // Exclude the root entry from count

    TraceResult {
        module: start_module,
        function: start_function,
        max_depth,
        direction: TraceDirection::Forward,
        total_items,
        entries,
    }
}

impl Execute for TraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let calls = trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.common.project,
            self.common.regex,
            self.depth,
            self.common.limit,
        )?;

        Ok(build_trace_result(
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
        let result = TraceResult::empty("TestModule".to_string(), "test_func".to_string(), 5, db::TraceDirection::Forward);
        assert_eq!(result.total_items, 0);
        assert_eq!(result.entries.len(), 0);
    }
}
