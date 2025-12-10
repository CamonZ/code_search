use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::queries::calls_from::find_calls_from;
use crate::types::Call;

/// A caller function with all its outgoing calls
#[derive(Debug, Clone, Serialize)]
pub struct CallerFunction {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub calls: Vec<Call>,
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
    /// Build grouped result from flat calls
    pub fn from_calls(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> Self {
        let total_calls = calls.len();

        // Group by module -> function -> calls
        // Using BTreeMap for consistent ordering
        let mut by_module: BTreeMap<String, (String, BTreeMap<CallerFunctionKey, Vec<Call>>)> =
            BTreeMap::new();

        for call in calls {
            let file = call.caller.file.clone().unwrap_or_default();
            let module_entry = by_module
                .entry(call.caller.module.clone())
                .or_insert_with(|| (file, BTreeMap::new()));

            let fn_key = CallerFunctionKey {
                name: call.caller.name.clone(),
                arity: call.caller.arity,
                kind: call.caller.kind.clone().unwrap_or_default(),
                start_line: call.caller.start_line.unwrap_or(0),
                end_line: call.caller.end_line.unwrap_or(0),
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
                        let mut seen: std::collections::HashSet<(String, String, i64)> =
                            std::collections::HashSet::new();
                        calls.retain(|c| {
                            seen.insert((
                                c.callee.module.clone(),
                                c.callee.name.clone(),
                                c.callee.arity,
                            ))
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
        let calls = find_calls_from(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(CallsFromResult::from_calls(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
