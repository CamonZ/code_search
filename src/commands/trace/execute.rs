use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::TraceCmd;
use crate::commands::Execute;
use crate::queries::trace::trace_calls;
use crate::types::Call;

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
    /// Build a tree structure from flat calls
    pub fn from_calls(
        start_module: String,
        start_function: String,
        max_depth: u32,
        calls: Vec<Call>,
    ) -> Self {
        if calls.is_empty() {
            return TraceResult {
                start_module,
                start_function,
                max_depth,
                total_calls: 0,
                roots: vec![],
            };
        }

        // Group calls by depth, then by caller
        let mut by_depth: HashMap<i64, Vec<&Call>> = HashMap::new();
        for call in &calls {
            if let Some(depth) = call.depth {
                by_depth.entry(depth).or_default().push(call);
            }
        }

        // Build depth 1 nodes (roots)
        let mut roots: Vec<TraceNode> = vec![];
        if let Some(depth1_calls) = by_depth.get(&1) {
            // Group by caller function
            let mut caller_map: HashMap<CallerKey, Vec<&Call>> = HashMap::new();
            for call in depth1_calls {
                let key = CallerKey {
                    module: call.caller.module.clone(),
                    function: call.caller.name.clone(),
                    arity: call.caller.arity,
                };
                caller_map.entry(key).or_default().push(call);
            }

            for (key, caller_calls) in caller_map {
                let first = caller_calls[0];
                let trace_calls: Vec<TraceCall> = caller_calls
                    .iter()
                    .map(|c| {
                        let children = build_children(
                            &by_depth,
                            2,
                            &c.callee.module,
                            &c.callee.name,
                            max_depth,
                        );
                        TraceCall {
                            module: c.callee.module.clone(),
                            function: c.callee.name.clone(),
                            arity: c.callee.arity,
                            line: c.line,
                            children,
                        }
                    })
                    .collect();

                roots.push(TraceNode {
                    module: key.module,
                    function: key.function,
                    arity: key.arity,
                    kind: first.caller.kind.clone().unwrap_or_default(),
                    start_line: first.caller.start_line.unwrap_or(0),
                    end_line: first.caller.end_line.unwrap_or(0),
                    file: first.caller.file.clone().unwrap_or_default(),
                    calls: trace_calls,
                });
            }
        }

        // Sort roots by module, function, arity
        roots.sort_by(|a, b| {
            (&a.module, &a.function, a.arity).cmp(&(&b.module, &b.function, b.arity))
        });

        // Count actual calls in the tree
        let total_calls = roots.iter().map(count_node_calls).sum();

        TraceResult {
            start_module,
            start_function,
            max_depth,
            total_calls,
            roots,
        }
    }
}

/// Count all calls in a node and its descendants
fn count_node_calls(node: &TraceNode) -> usize {
    let own_calls = node.calls.len();
    let child_calls: usize = node
        .calls
        .iter()
        .flat_map(|c| &c.children)
        .map(count_node_calls)
        .sum();
    own_calls + child_calls
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallerKey {
    module: String,
    function: String,
    arity: i64,
}

/// Recursively build children nodes for a callee
fn build_children(
    by_depth: &HashMap<i64, Vec<&Call>>,
    depth: i64,
    parent_module: &str,
    parent_function: &str,
    max_depth: u32,
) -> Vec<TraceNode> {
    if depth > max_depth as i64 {
        return vec![];
    }

    let Some(calls) = by_depth.get(&depth) else {
        return vec![];
    };

    // Find calls where the caller matches the parent callee
    let matching: Vec<_> = calls
        .iter()
        .filter(|c| c.caller.module == parent_module && c.caller.name == parent_function)
        .collect();

    if matching.is_empty() {
        return vec![];
    }

    // Group by caller
    let mut caller_map: HashMap<CallerKey, Vec<&&Call>> = HashMap::new();
    for call in &matching {
        let key = CallerKey {
            module: call.caller.module.clone(),
            function: call.caller.name.clone(),
            arity: call.caller.arity,
        };
        caller_map.entry(key).or_default().push(call);
    }

    let mut nodes: Vec<TraceNode> = vec![];
    for (key, caller_calls) in caller_map {
        let first = caller_calls[0];
        let trace_calls: Vec<TraceCall> = caller_calls
            .iter()
            .map(|c| {
                let children = build_children(
                    by_depth,
                    depth + 1,
                    &c.callee.module,
                    &c.callee.name,
                    max_depth,
                );
                TraceCall {
                    module: c.callee.module.clone(),
                    function: c.callee.name.clone(),
                    arity: c.callee.arity,
                    line: c.line,
                    children,
                }
            })
            .collect();

        nodes.push(TraceNode {
            module: key.module,
            function: key.function,
            arity: key.arity,
            kind: first.caller.kind.clone().unwrap_or_default(),
            start_line: first.caller.start_line.unwrap_or(0),
            end_line: first.caller.end_line.unwrap_or(0),
            file: first.caller.file.clone().unwrap_or_default(),
            calls: trace_calls,
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
