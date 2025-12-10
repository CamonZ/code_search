use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::ReverseTraceCmd;
use crate::commands::Execute;
use crate::queries::reverse_trace::{reverse_trace_calls, ReverseTraceStep};

/// A call target in the reverse trace tree (the callee being called)
#[derive(Debug, Clone, Serialize)]
pub struct ReverseTraceTarget {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub line: i64,
}

/// A node in the reverse trace tree (a caller function)
#[derive(Debug, Clone, Serialize)]
pub struct ReverseTraceNode {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub targets: Vec<ReverseTraceTarget>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callers: Vec<ReverseTraceNode>,
}

/// Result of the reverse-trace command execution
#[derive(Debug, Default, Serialize)]
pub struct ReverseTraceResult {
    pub target_module: String,
    pub target_function: String,
    pub max_depth: u32,
    pub total_callers: usize,
    pub roots: Vec<ReverseTraceNode>,
}

impl ReverseTraceResult {
    /// Build a tree structure from flat trace steps
    pub fn from_steps(
        target_module: String,
        target_function: String,
        max_depth: u32,
        steps: Vec<ReverseTraceStep>,
    ) -> Self {
        let total_callers = steps.len();

        if steps.is_empty() {
            return ReverseTraceResult {
                target_module,
                target_function,
                max_depth,
                total_callers: 0,
                roots: vec![],
            };
        }

        // Group steps by depth, then by caller
        let mut by_depth: HashMap<i64, Vec<&ReverseTraceStep>> = HashMap::new();
        for step in &steps {
            by_depth.entry(step.depth).or_default().push(step);
        }

        // Build depth 1 nodes (roots - direct callers of the target)
        let mut roots: Vec<ReverseTraceNode> = vec![];
        if let Some(depth1_steps) = by_depth.get(&1) {
            // Group by caller function
            let mut caller_map: HashMap<CallerKey, Vec<&ReverseTraceStep>> = HashMap::new();
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
                let targets: Vec<ReverseTraceTarget> = caller_steps
                    .iter()
                    .map(|s| ReverseTraceTarget {
                        module: s.callee_module.clone(),
                        function: s.callee_function.clone(),
                        arity: s.callee_arity,
                        line: s.line,
                    })
                    .collect();

                let callers = build_callers(&by_depth, 2, &key.module, &key.function, key.arity, max_depth);

                roots.push(ReverseTraceNode {
                    module: key.module,
                    function: key.function,
                    arity: key.arity,
                    kind: first.caller_kind.clone(),
                    start_line: first.caller_start_line,
                    end_line: first.caller_end_line,
                    file: first.file.clone(),
                    targets,
                    callers,
                });
            }
        }

        // Sort roots by module, function, arity
        roots.sort_by(|a, b| {
            (&a.module, &a.function, a.arity).cmp(&(&b.module, &b.function, b.arity))
        });

        ReverseTraceResult {
            target_module,
            target_function,
            max_depth,
            total_callers,
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

/// Recursively build caller nodes for a function
fn build_callers(
    by_depth: &HashMap<i64, Vec<&ReverseTraceStep>>,
    depth: i64,
    target_module: &str,
    target_function: &str,
    target_arity: i64,
    max_depth: u32,
) -> Vec<ReverseTraceNode> {
    if depth > max_depth as i64 {
        return vec![];
    }

    let Some(steps) = by_depth.get(&depth) else {
        return vec![];
    };

    // Find steps where the callee matches the target (i.e., callers of the target)
    let matching: Vec<_> = steps
        .iter()
        .filter(|s| {
            s.callee_module == target_module
                && s.callee_function == target_function
                && s.callee_arity == target_arity
        })
        .collect();

    if matching.is_empty() {
        return vec![];
    }

    // Group by caller
    let mut caller_map: HashMap<CallerKey, Vec<&&ReverseTraceStep>> = HashMap::new();
    for step in &matching {
        let key = CallerKey {
            module: step.caller_module.clone(),
            function: step.caller_function.clone(),
            arity: step.caller_arity,
        };
        caller_map.entry(key).or_default().push(step);
    }

    let mut nodes: Vec<ReverseTraceNode> = vec![];
    for (key, caller_steps) in caller_map {
        let first = caller_steps[0];
        let targets: Vec<ReverseTraceTarget> = caller_steps
            .iter()
            .map(|s| ReverseTraceTarget {
                module: s.callee_module.clone(),
                function: s.callee_function.clone(),
                arity: s.callee_arity,
                line: s.line,
            })
            .collect();

        let callers = build_callers(by_depth, depth + 1, &key.module, &key.function, key.arity, max_depth);

        nodes.push(ReverseTraceNode {
            module: key.module,
            function: key.function,
            arity: key.arity,
            kind: first.caller_kind.clone(),
            start_line: first.caller_start_line,
            end_line: first.caller_end_line,
            file: first.file.clone(),
            targets,
            callers,
        });
    }

    nodes.sort_by(|a, b| {
        (&a.module, &a.function, a.arity).cmp(&(&b.module, &b.function, b.arity))
    });

    nodes
}

impl Execute for ReverseTraceCmd {
    type Output = ReverseTraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let steps = reverse_trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.depth,
            self.limit,
        )?;

        Ok(ReverseTraceResult::from_steps(
            self.module,
            self.function,
            self.depth,
            steps,
        ))
    }
}
