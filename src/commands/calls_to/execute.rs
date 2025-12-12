use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::CallsToCmd;
use crate::commands::Execute;
use crate::dedup::sort_and_deduplicate;
use crate::queries::calls_to::find_calls_to;
use crate::types::{Call, ModuleGroupResult};
use crate::utils::convert_to_module_groups;

/// A callee function (target) with all its callers
#[derive(Debug, Clone, Serialize)]
pub struct CalleeFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
}

impl ModuleGroupResult<CalleeFunction> {
    /// Build grouped result from flat calls
    pub fn from_calls(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> Self {
        let total_items = calls.len();

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

        // Convert to ModuleGroup structure
        let items = convert_to_module_groups(
            by_module,
            |key, mut callers| {
                // Deduplicate callers, keeping first occurrence by line
                sort_and_deduplicate(
                    &mut callers,
                    |c| (c.caller.module.clone(), c.caller.name.clone(), c.caller.arity, c.line),
                    |c| (c.caller.module.clone(), c.caller.name.clone(), c.caller.arity),
                );

                CalleeFunction {
                    name: key.name,
                    arity: key.arity,
                    callers,
                }
            },
            // File is intentionally empty because callees are the grouping key,
            // and a module can be defined across multiple files. The calls themselves
            // carry file information where needed.
            |_module, _map| String::new(),
        );

        ModuleGroupResult {
            module_pattern,
            function_pattern: Some(function_pattern),
            total_items,
            items,
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
    type Output = ModuleGroupResult<CalleeFunction>;

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

        Ok(<ModuleGroupResult<CalleeFunction>>::from_calls(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
