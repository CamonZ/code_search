use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::TraceCmd;
use crate::commands::Execute;
use crate::queries::trace::{trace_calls, TraceStep};

/// A call target in the trace tree
#[derive(Debug, Clone, Serialize)]
pub struct TraceCall {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub line: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TraceNode>,
}

/// A node in the trace tree (a caller function)
#[derive(Debug, Clone, Serialize)]
pub struct TraceNode {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub calls: Vec<TraceCall>,
}

/// Result of the trace command execution
#[derive(Debug, Default, Serialize)]
pub struct TraceResult {
    pub start_module: String,
    pub start_function: String,
    pub max_depth: u32,
    pub total_calls: usize,
    pub roots: Vec<TraceNode>,
}

impl TraceResult {
    /// Build a tree structure from flat trace steps
    pub fn from_steps(
        start_module: String,
        start_function: String,
        max_depth: u32,
        steps: Vec<TraceStep>,
    ) -> Self {
        let total_calls = steps.len();

        if steps.is_empty() {
            return TraceResult {
                start_module,
                start_function,
                max_depth,
                total_calls: 0,
                roots: vec![],
            };
        }

        // Group steps by depth, then by caller
        let mut by_depth: HashMap<i64, Vec<&TraceStep>> = HashMap::new();
        for step in &steps {
            by_depth.entry(step.depth).or_default().push(step);
        }

        // Build depth 1 nodes (roots)
        let mut roots: Vec<TraceNode> = vec![];
        if let Some(depth1_steps) = by_depth.get(&1) {
            // Group by caller function
            let mut caller_map: HashMap<CallerKey, Vec<&TraceStep>> = HashMap::new();
            for step in depth1_steps {
                let key = CallerKey {
                    module: step.caller_module.clone(),
                    function: step.caller_function.clone(),
                    arity: step.caller_arity,
                };
                caller_map.entry(key).or_default().push(step);
            }

            for (key, caller_steps) in caller_map {
                let first = caller_steps[0];
                let calls: Vec<TraceCall> = caller_steps
                    .iter()
                    .map(|s| {
                        let children = build_children(&by_depth, 2, &s.callee_module, &s.callee_function, max_depth);
                        TraceCall {
                            module: s.callee_module.clone(),
                            function: s.callee_function.clone(),
                            arity: s.callee_arity,
                            line: s.line,
                            children,
                        }
                    })
                    .collect();

                roots.push(TraceNode {
                    module: key.module,
                    function: key.function,
                    arity: key.arity,
                    kind: first.caller_kind.clone(),
                    start_line: first.caller_start_line,
                    end_line: first.caller_end_line,
                    file: first.file.clone(),
                    calls,
                });
            }
        }

        // Sort roots by module, function, arity
        roots.sort_by(|a, b| {
            (&a.module, &a.function, a.arity).cmp(&(&b.module, &b.function, b.arity))
        });

        TraceResult {
            start_module,
            start_function,
            max_depth,
            total_calls,
            roots,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallerKey {
    module: String,
    function: String,
    arity: i64,
}

/// Recursively build children nodes for a callee
fn build_children(
    by_depth: &HashMap<i64, Vec<&TraceStep>>,
    depth: i64,
    parent_module: &str,
    parent_function: &str,
    max_depth: u32,
) -> Vec<TraceNode> {
    if depth > max_depth as i64 {
        return vec![];
    }

    let Some(steps) = by_depth.get(&depth) else {
        return vec![];
    };

    // Find steps where the caller matches the parent callee
    let matching: Vec<_> = steps
        .iter()
        .filter(|s| s.caller_module == parent_module && s.caller_function == parent_function)
        .collect();

    if matching.is_empty() {
        return vec![];
    }

    // Group by caller
    let mut caller_map: HashMap<CallerKey, Vec<&&TraceStep>> = HashMap::new();
    for step in &matching {
        let key = CallerKey {
            module: step.caller_module.clone(),
            function: step.caller_function.clone(),
            arity: step.caller_arity,
        };
        caller_map.entry(key).or_default().push(step);
    }

    let mut nodes: Vec<TraceNode> = vec![];
    for (key, caller_steps) in caller_map {
        let first = caller_steps[0];
        let calls: Vec<TraceCall> = caller_steps
            .iter()
            .map(|s| {
                let children = build_children(by_depth, depth + 1, &s.callee_module, &s.callee_function, max_depth);
                TraceCall {
                    module: s.callee_module.clone(),
                    function: s.callee_function.clone(),
                    arity: s.callee_arity,
                    line: s.line,
                    children,
                }
            })
            .collect();

        nodes.push(TraceNode {
            module: key.module,
            function: key.function,
            arity: key.arity,
            kind: first.caller_kind.clone(),
            start_line: first.caller_start_line,
            end_line: first.caller_end_line,
            file: first.file.clone(),
            calls,
        });
    }

    nodes.sort_by(|a, b| {
        (&a.module, &a.function, a.arity).cmp(&(&b.module, &b.function, b.arity))
    });

    nodes
}

impl Execute for TraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let steps = trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.depth,
            self.limit,
        )?;

        Ok(TraceResult::from_steps(
            self.module,
            self.function,
            self.depth,
            steps,
        ))
    }
}
