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
    use db::types::FunctionRef;

    #[test]
    fn test_empty_trace() {
        let result = TraceResult::empty("TestModule".to_string(), "test_func".to_string(), 5, db::TraceDirection::Forward);
        assert_eq!(result.total_items, 0);
        assert_eq!(result.entries.len(), 0);
    }

    /// Helper to build a Call at a given depth with caller and callee details.
    fn make_call(
        caller_module: &str,
        caller_name: &str,
        caller_arity: i64,
        callee_module: &str,
        callee_name: &str,
        callee_arity: i64,
        line: i64,
        depth: i64,
    ) -> Call {
        Call {
            caller: FunctionRef::new(caller_module, caller_name, caller_arity),
            callee: FunctionRef::new(callee_module, callee_name, callee_arity),
            line,
            call_type: None,
            depth: Some(depth),
        }
    }

    // =========================================================================
    // Deduplication tests for depth-1 conditional (lines 52-60)
    // These kill mutants where == is replaced with != and && with ||
    // =========================================================================

    /// Duplicate calls at depth 1 with identical (module, function, arity) must
    /// produce exactly one entry. Kills mutants: `== -> !=` on depth, module,
    /// function, and arity checks (lines 53-56), plus `&& -> ||` on line 60.
    #[test]
    fn test_duplicate_calls_at_depth_1_are_deduplicated() {
        let calls = vec![
            make_call("Start", "go", 0, "Target", "run", 1, 10, 1),
            make_call("Start", "go", 0, "Target", "run", 1, 15, 1), // duplicate
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);
        // Root + exactly 1 callee (the duplicate should be filtered out)
        assert_eq!(result.entries.len(), 2, "duplicate depth-1 call should be deduplicated");
        assert_eq!(result.total_items, 1);
        assert_eq!(result.entries[1].module, "Target");
        assert_eq!(result.entries[1].function, "run");
        assert_eq!(result.entries[1].arity, 1);
    }

    /// Two calls at depth 1 that differ ONLY in module must produce two entries.
    /// Kills mutant: `&& -> ||` on the module comparison (line 54).
    #[test]
    fn test_different_module_at_depth_1_not_deduplicated() {
        let calls = vec![
            make_call("Start", "go", 0, "Alpha", "run", 1, 10, 1),
            make_call("Start", "go", 0, "Beta", "run", 1, 15, 1),
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);
        // Root + 2 distinct callees
        assert_eq!(result.entries.len(), 3, "different modules should produce separate entries");
        assert_eq!(result.total_items, 2);
        assert_eq!(result.entries[1].module, "Alpha");
        assert_eq!(result.entries[2].module, "Beta");
    }

    /// Two calls at depth 1 that differ ONLY in function name must produce two
    /// entries. Kills mutant: `&& -> ||` on the function comparison (line 55).
    #[test]
    fn test_different_function_at_depth_1_not_deduplicated() {
        let calls = vec![
            make_call("Start", "go", 0, "Target", "run", 1, 10, 1),
            make_call("Start", "go", 0, "Target", "walk", 1, 15, 1),
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);
        assert_eq!(result.entries.len(), 3, "different functions should produce separate entries");
        assert_eq!(result.total_items, 2);
        assert_eq!(result.entries[1].function, "run");
        assert_eq!(result.entries[2].function, "walk");
    }

    /// Two calls at depth 1 that differ ONLY in arity must produce two entries.
    /// Kills mutant: `== -> !=` on the arity comparison (line 56).
    #[test]
    fn test_different_arity_at_depth_1_not_deduplicated() {
        let calls = vec![
            make_call("Start", "go", 0, "Target", "run", 1, 10, 1),
            make_call("Start", "go", 0, "Target", "run", 2, 15, 1),
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);
        assert_eq!(result.entries.len(), 3, "different arities should produce separate entries");
        assert_eq!(result.total_items, 2);
        assert_eq!(result.entries[1].arity, 1);
        assert_eq!(result.entries[2].arity, 2);
    }

    /// Multiple distinct callees at depth 1 followed by a duplicate. Ensures the
    /// `|| -> &&` mutant on line 59 is killed: that mutant causes
    /// `seen_at_depth.insert(usize::MAX)` to block subsequent new entries after the
    /// first one.
    #[test]
    fn test_multiple_distinct_callees_at_depth_1() {
        let calls = vec![
            make_call("Start", "go", 0, "Alpha", "a", 0, 10, 1),
            make_call("Start", "go", 0, "Beta", "b", 0, 20, 1),
            make_call("Start", "go", 0, "Gamma", "c", 0, 30, 1),
            make_call("Start", "go", 0, "Alpha", "a", 0, 40, 1), // duplicate of first
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);
        // Root + 3 distinct callees (Alpha, Beta, Gamma); the duplicate Alpha is filtered
        assert_eq!(result.entries.len(), 4, "should have root + 3 distinct callees");
        assert_eq!(result.total_items, 3);
    }

    // =========================================================================
    // Integration: run() through formatted output
    // =========================================================================

    /// Test build_trace_result with a full call chain (depth 1 + depth 2) to verify
    /// the result metadata is correctly assembled and can be formatted.
    #[test]
    fn test_build_trace_result_formats_through_output() {
        use crate::output::Outputable;

        let calls = vec![
            make_call("Start", "go", 0, "Mid", "step", 1, 10, 1),
            make_call("Mid", "step", 1, "End", "done", 0, 20, 2),
        ];
        let result = build_trace_result("Start".into(), "go".into(), 5, calls);

        assert_eq!(result.module, "Start");
        assert_eq!(result.function, "go");
        assert_eq!(result.max_depth, 5);
        assert!(matches!(result.direction, TraceDirection::Forward));
        assert_eq!(result.total_items, 2);
        assert_eq!(result.entries.len(), 3); // root + 2 callees

        let table = result.to_table();
        assert!(table.contains("Trace from: Start.go"), "output should include header");
        assert!(table.contains("Found 2 call(s) in chain:"), "output should include count");
        assert!(table.contains("Mid.step/1"), "output should include depth-1 callee");
        assert!(table.contains("End.done/0"), "output should include depth-2 callee");
    }

    /// Verify that an empty call list returns an empty TraceResult (via the early return).
    #[test]
    fn test_empty_calls_returns_empty_result() {
        let result = build_trace_result("Mod".into(), "func".into(), 3, vec![]);
        assert_eq!(result.total_items, 0);
        assert!(result.entries.is_empty());
        assert_eq!(result.module, "Mod");
        assert_eq!(result.function, "func");
        assert_eq!(result.max_depth, 3);
    }
}
