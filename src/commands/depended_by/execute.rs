use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::DependedByCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::depended_by::find_dependents;
use crate::types::{Call, ModuleGroupResult, ModuleGroup};

/// A target function being called in the dependency module
#[derive(Debug, Clone, Serialize)]
pub struct DependentTarget {
    pub function: String,
    pub arity: i64,
    pub line: i64,
}

/// A caller function in a dependent module
#[derive(Debug, Clone, Serialize)]
pub struct DependentCaller {
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub targets: Vec<DependentTarget>,
}

impl ModuleGroupResult<DependentCaller> {
    /// Build a grouped structure from flat calls
    pub fn from_calls(target_module: String, calls: Vec<Call>) -> Self {
        let total_items = calls.len();

        if calls.is_empty() {
            return ModuleGroupResult {
                module_pattern: target_module,
                function_pattern: None,
                total_items: 0,
                items: vec![],
            };
        }

        // Group by caller_module -> caller_function -> targets
        // Using BTreeMap for automatic sorting by module and function key
        let mut by_module: BTreeMap<String, BTreeMap<(String, i64), Vec<&Call>>> = BTreeMap::new();
        for call in &calls {
            by_module
                .entry(call.caller.module.to_string())
                .or_default()
                .entry((call.caller.name.to_string(), call.caller.arity))
                .or_default()
                .push(call);
        }

        let items: Vec<ModuleGroup<DependentCaller>> = by_module
            .into_iter()
            .map(|(module_name, callers_map)| {
                // Determine module file from first caller in first function
                let module_file = callers_map
                    .values()
                    .next()
                    .and_then(|calls| calls.first())
                    .and_then(|call| call.caller.file.as_deref())
                    .unwrap_or("")
                    .to_string();

                let entries: Vec<DependentCaller> = callers_map
                    .into_iter()
                    .map(|((func_name, arity), func_calls)| {
                        let first = func_calls[0];

                        let targets: Vec<DependentTarget> = func_calls
                            .iter()
                            .map(|c| DependentTarget {
                                function: c.callee.name.to_string(),
                                arity: c.callee.arity,
                                line: c.line,
                            })
                            .collect();

                        DependentCaller {
                            function: func_name,
                            arity,
                            kind: first.caller.kind.as_deref().unwrap_or("").to_string(),
                            start_line: first.caller.start_line.unwrap_or(0),
                            end_line: first.caller.end_line.unwrap_or(0),
                            file: first.caller.file.as_deref().unwrap_or("").to_string(),
                            targets,
                        }
                    })
                    .collect();

                ModuleGroup {
                    name: module_name,
                    file: module_file,
                    entries,
                    function_count: None,
                }
            })
            .collect();

        ModuleGroupResult {
            module_pattern: target_module,
            function_pattern: None,
            total_items,
            items,
        }
    }
}

impl Execute for DependedByCmd {
    type Output = ModuleGroupResult<DependentCaller>;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependents(
            db,
            &self.module,
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(<ModuleGroupResult<DependentCaller>>::from_calls(self.module, calls))
    }
}
