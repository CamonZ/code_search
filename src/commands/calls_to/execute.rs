use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsToCmd;
use crate::commands::Execute;
use crate::queries::calls_to::{find_calls_to, CallEdge};

/// A caller that invokes a target function
#[derive(Debug, Clone, Serialize)]
pub struct Caller {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub line: i64,
    pub call_type: String,
}

/// A callee function (target) with all its callers
#[derive(Debug, Clone, Serialize)]
pub struct CalleeFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Caller>,
}

/// A module containing callee functions
#[derive(Debug, Clone, Serialize)]
pub struct CalleeModule {
    pub name: String,
    pub functions: Vec<CalleeFunction>,
}

/// Result of the calls-to command execution
#[derive(Debug, Default, Serialize)]
pub struct CallsToResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub total_calls: usize,
    pub modules: Vec<CalleeModule>,
}

impl CallsToResult {
    /// Build grouped result from flat call edges
    pub fn from_edges(module_pattern: String, function_pattern: String, edges: Vec<CallEdge>) -> Self {
        let total_calls = edges.len();

        // Group by callee module -> callee function -> callers
        let mut by_module: BTreeMap<String, BTreeMap<CalleeFunctionKey, Vec<Caller>>> = BTreeMap::new();

        for edge in edges {
            let fn_key = CalleeFunctionKey {
                name: edge.callee_function.clone(),
                arity: edge.callee_arity,
            };

            let caller = Caller {
                module: edge.caller_module,
                function: edge.caller_function,
                arity: edge.caller_arity,
                kind: edge.caller_kind,
                start_line: edge.caller_start_line,
                end_line: edge.caller_end_line,
                file: edge.file,
                line: edge.line,
                call_type: edge.call_type,
            };

            by_module
                .entry(edge.callee_module)
                .or_default()
                .entry(fn_key)
                .or_default()
                .push(caller);
        }

        // Convert to Vec structure
        let modules: Vec<CalleeModule> = by_module
            .into_iter()
            .map(|(module_name, functions_map)| {
                let functions: Vec<CalleeFunction> = functions_map
                    .into_iter()
                    .map(|(key, mut callers)| {
                        // Deduplicate callers, keeping first occurrence by line
                        callers.sort_by_key(|c| (c.module.clone(), c.function.clone(), c.arity, c.line));
                        let mut seen: std::collections::HashSet<(String, String, i64)> = std::collections::HashSet::new();
                        callers.retain(|c| {
                            seen.insert((c.module.clone(), c.function.clone(), c.arity))
                        });

                        CalleeFunction {
                            name: key.name,
                            arity: key.arity,
                            callers,
                        }
                    })
                    .collect();

                CalleeModule {
                    name: module_name,
                    functions,
                }
            })
            .collect();

        CallsToResult {
            module_pattern,
            function_pattern,
            total_calls,
            modules,
        }
    }
}

/// Key for grouping by callee function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CalleeFunctionKey {
    name: String,
    arity: i64,
}

impl Execute for CallsToCmd {
    type Output = CallsToResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let edges = find_calls_to(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(CallsToResult::from_edges(
            self.module,
            self.function.unwrap_or_default(),
            edges,
        ))
    }
}
