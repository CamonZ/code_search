use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::DependedByCmd;
use crate::commands::Execute;
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
        let mut by_module: HashMap<String, HashMap<(String, i64), Vec<&Call>>> = HashMap::new();
        for call in &calls {
            by_module
                .entry(call.caller.module.clone())
                .or_default()
                .entry((call.caller.name.clone(), call.caller.arity))
                .or_default()
                .push(call);
        }

        let mut items: Vec<ModuleGroup<DependentCaller>> = vec![];
        for (module_name, callers_map) in by_module {
            let mut entries: Vec<DependentCaller> = vec![];
            let mut module_file = String::new();

            for ((func_name, arity), func_calls) in callers_map {
                let first = func_calls[0];

                // Track the first file we encounter for this module
                if module_file.is_empty() {
                    module_file = first.caller.file.clone().unwrap_or_default();
                }

                let targets: Vec<DependentTarget> = func_calls
                    .iter()
                    .map(|c| DependentTarget {
                        function: c.callee.name.clone(),
                        arity: c.callee.arity,
                        line: c.line,
                    })
                    .collect();

                entries.push(DependentCaller {
                    function: func_name,
                    arity,
                    kind: first.caller.kind.clone().unwrap_or_default(),
                    start_line: first.caller.start_line.unwrap_or(0),
                    end_line: first.caller.end_line.unwrap_or(0),
                    file: first.caller.file.clone().unwrap_or_default(),
                    targets,
                });
            }

            // Sort callers by function, arity
            entries.sort_by(|a, b| (&a.function, a.arity).cmp(&(&b.function, b.arity)));

            items.push(ModuleGroup {
                name: module_name,
                file: module_file,
                entries,
            });
        }

        // Sort modules by name
        items.sort_by(|a, b| a.name.cmp(&b.name));

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

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependents(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(<ModuleGroupResult<DependentCaller>>::from_calls(self.module, calls))
    }
}
