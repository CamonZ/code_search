use std::error::Error;

use serde::Serialize;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::queries::calls_from::find_calls_from;
use crate::types::{Call, ModuleGroupResult};
use crate::utils::group_calls;

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
        let (total_items, items) = group_calls(
            calls,
            // Group by caller module
            |call| call.caller.module.to_string(),
            // Key by caller function metadata
            |call| CallerFunctionKey {
                name: call.caller.name.to_string(),
                arity: call.caller.arity,
                kind: call.caller.kind.as_deref().unwrap_or("").to_string(),
                start_line: call.caller.start_line.unwrap_or(0),
                end_line: call.caller.end_line.unwrap_or(0),
            },
            // Sort by line number
            |a, b| a.line.cmp(&b.line),
            // Deduplicate by callee (module, name, arity)
            |c| (c.callee.module.to_string(), c.callee.name.to_string(), c.callee.arity),
            // Build CallerFunction entry
            |key, calls| CallerFunction {
                name: key.name,
                arity: key.arity,
                kind: key.kind,
                start_line: key.start_line,
                end_line: key.end_line,
                calls,
            },
            // File tracking strategy: extract from first call in first function
            |_module, functions_map| {
                functions_map
                    .values()
                    .next()
                    .and_then(|calls| calls.first())
                    .and_then(|call| call.caller.file.as_deref())
                    .unwrap_or("")
                    .to_string()
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
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(<ModuleGroupResult<CallerFunction>>::from_calls(
            self.module,
            self.function.unwrap_or_default(),
            calls,
        ))
    }
}
