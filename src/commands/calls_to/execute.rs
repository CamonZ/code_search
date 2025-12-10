use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsToCmd;
use crate::commands::Execute;
use crate::queries::calls_to::find_calls_to;
use crate::types::Call;

/// A callee function (target) with all its callers
#[derive(Debug, Clone, Serialize)]
pub struct CalleeFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
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
    /// Build grouped result from flat calls
    pub fn from_calls(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> Self {
        let total_calls = calls.len();

        // Group by callee module -> callee function -> callers
        let mut by_module: BTreeMap<String, BTreeMap<CalleeFunctionKey, Vec<Call>>> =
            BTreeMap::new();

        for call in calls {
            let fn_key = CalleeFunctionKey {
                name: call.callee.name.clone(),
                arity: call.callee.arity,
            };

            by_module
                .entry(call.callee.module.clone())
                .or_default()
                .entry(fn_key)
                .or_default()
                .push(call);
        }

        // Convert to Vec structure
        let modules: Vec<CalleeModule> = by_module
            .into_iter()
            .map(|(module_name, functions_map)| {
                let functions: Vec<CalleeFunction> = functions_map
                    .into_iter()
                    .map(|(key, mut callers)| {
                        // Deduplicate callers, keeping first occurrence by line
                        callers.sort_by_key(|c| {
                            (
                                c.caller.module.clone(),
                                c.caller.name.clone(),
                                c.caller.arity,
                                c.line,
                            )
                        });
                        let mut seen: std::collections::HashSet<(String, String, i64)> =
                            std::collections::HashSet::new();
                        callers.retain(|c| {
                            seen.insert((
                                c.caller.module.clone(),
                                c.caller.name.clone(),
                                c.caller.arity,
                            ))
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
        let calls = find_calls_to(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(CallsToResult::from_calls(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
