use std::error::Error;

use serde::Serialize;

use super::CallsToCmd;
use crate::commands::Execute;
use db::queries::calls_to::find_calls_to;
use db::types::{Call, ModuleGroupResult};
use crate::utils::group_calls;

/// A callee function (target) with all its callers
#[derive(Debug, Clone, Serialize)]
pub struct CalleeFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
}

/// Key for grouping by callee function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CalleeFunctionKey {
    name: String,
    arity: i64,
}

/// Build grouped result from flat calls
fn build_callee_result(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> ModuleGroupResult<CalleeFunction> {
    let (total_items, items) = group_calls(
        calls,
        // Group by callee module
        |call| call.callee.module.to_string(),
        // Key by callee function metadata
        |call| CalleeFunctionKey {
            name: call.callee.name.to_string(),
            arity: call.callee.arity,
        },
        // Sort by caller module, name, arity, then line
        |a, b| {
            a.caller.module.as_ref().cmp(b.caller.module.as_ref())
                .then_with(|| a.caller.name.as_ref().cmp(b.caller.name.as_ref()))
                .then_with(|| a.caller.arity.cmp(&b.caller.arity))
                .then_with(|| a.line.cmp(&b.line))
        },
        // Deduplicate by caller (module, name, arity)
        |c| (c.caller.module.to_string(), c.caller.name.to_string(), c.caller.arity),
        // Build CalleeFunction entry
        |key, callers| CalleeFunction {
            name: key.name,
            arity: key.arity,
            callers,
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

impl Execute for CallsToCmd {
    type Output = ModuleGroupResult<CalleeFunction>;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_calls_to(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(build_callee_result(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
