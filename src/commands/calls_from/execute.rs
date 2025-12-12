use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::dedup::sort_and_deduplicate;
use crate::queries::calls_from::find_calls_from;
use crate::types::{Call, ModuleGroupResult};
use crate::utils::convert_to_module_groups;

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

impl ModuleGroupResult<CallerFunction> {
    /// Build grouped result from flat calls
    pub fn from_calls(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> Self {
        let total_items = calls.len();

        // Group by module -> function -> calls
        // Using BTreeMap for consistent ordering
        let mut by_module: BTreeMap<String, BTreeMap<CallerFunctionKey, Vec<Call>>> = BTreeMap::new();

        for call in calls {
            let fn_key = CallerFunctionKey {
                name: call.caller.name.clone(),
                arity: call.caller.arity,
                kind: call.caller.kind.clone().unwrap_or_default(),
                start_line: call.caller.start_line.unwrap_or(0),
                end_line: call.caller.end_line.unwrap_or(0),
            };

            by_module
                .entry(call.caller.module.clone())
                .or_default()
                .entry(fn_key)
                .or_default()
                .push(call);
        }

        // Convert to ModuleGroup structure
        let items = convert_to_module_groups(
            by_module,
            |key, mut calls| {
                // Deduplicate calls, keeping first occurrence by line
                sort_and_deduplicate(
                    &mut calls,
                    |c| c.line,
                    |c| (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity),
                );

                CallerFunction {
                    name: key.name,
                    arity: key.arity,
                    kind: key.kind,
                    start_line: key.start_line,
                    end_line: key.end_line,
                    calls,
                }
            },
            // File tracking strategy: extract from first call in first function
            |_module, functions_map| {
                functions_map
                    .values()
                    .next()
                    .and_then(|calls| calls.first())
                    .and_then(|call| call.caller.file.clone())
                    .unwrap_or_default()
            },
        );

        ModuleGroupResult {
            module_pattern,
            function_pattern: Some(function_pattern),
            total_items,
            items,
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
    type Output = ModuleGroupResult<CallerFunction>;

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

        Ok(<ModuleGroupResult<CallerFunction>>::from_calls(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
