use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::queries::calls_from::{find_calls_from, CallEdge};

/// A single outgoing call from a function
#[derive(Debug, Clone, Serialize)]
pub struct CallTarget {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub line: i64,
    pub call_type: String,
}

/// A caller function with all its outgoing calls
#[derive(Debug, Clone, Serialize)]
pub struct CallerFunction {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub calls: Vec<CallTarget>,
}

/// A module with all its caller functions
#[derive(Debug, Clone, Serialize)]
pub struct CallerModule {
    pub name: String,
    pub file: String,
    pub functions: Vec<CallerFunction>,
}

/// Result of the calls-from command execution
#[derive(Debug, Default, Serialize)]
pub struct CallsFromResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub total_calls: usize,
    pub modules: Vec<CallerModule>,
}

impl CallsFromResult {
    /// Build grouped result from flat call edges
    pub fn from_edges(module_pattern: String, function_pattern: String, edges: Vec<CallEdge>) -> Self {
        let total_calls = edges.len();

        // Group by module -> function -> calls
        // Using BTreeMap for consistent ordering
        let mut by_module: BTreeMap<String, (String, BTreeMap<CallerFunctionKey, Vec<CallTarget>>)> = BTreeMap::new();

        for edge in edges {
            let module_entry = by_module
                .entry(edge.caller_module.clone())
                .or_insert_with(|| (edge.file.clone(), BTreeMap::new()));

            let fn_key = CallerFunctionKey {
                name: edge.caller_function.clone(),
                arity: edge.caller_arity,
                kind: edge.caller_kind.clone(),
                start_line: edge.caller_start_line,
                end_line: edge.caller_end_line,
            };

            let call = CallTarget {
                module: edge.callee_module,
                function: edge.callee_function,
                arity: edge.callee_arity,
                line: edge.line,
                call_type: edge.call_type,
            };

            module_entry.1.entry(fn_key).or_default().push(call);
        }

        // Convert to Vec structure
        let modules: Vec<CallerModule> = by_module
            .into_iter()
            .map(|(module_name, (file, functions_map))| {
                let functions: Vec<CallerFunction> = functions_map
                    .into_iter()
                    .map(|(key, mut calls)| {
                        // Deduplicate calls, keeping first occurrence by line
                        calls.sort_by_key(|c| c.line);
                        let mut seen: std::collections::HashSet<(String, String, i64)> = std::collections::HashSet::new();
                        calls.retain(|c| {
                            seen.insert((c.module.clone(), c.function.clone(), c.arity))
                        });

                        CallerFunction {
                            name: key.name,
                            arity: key.arity,
                            kind: key.kind,
                            start_line: key.start_line,
                            end_line: key.end_line,
                            calls,
                        }
                    })
                    .collect();

                CallerModule {
                    name: module_name,
                    file,
                    functions,
                }
            })
            .collect();

        CallsFromResult {
            module_pattern,
            function_pattern,
            total_calls,
            modules,
        }
    }
}

/// Key for grouping by caller function (used internally)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CallerFunctionKey {
    name: String,
    arity: i64,
    kind: String,
    start_line: i64,
    end_line: i64,
}

impl Execute for CallsFromCmd {
    type Output = CallsFromResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let edges = find_calls_from(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(CallsFromResult::from_edges(
            self.module,
            self.function.unwrap_or_default(),
            edges,
        ))
    }
}
