use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::queries::depends_on::find_dependencies;
use crate::types::{Call, ModuleGroupResult, ModuleGroup};

/// A function in a dependency module being called
#[derive(Debug, Clone, Serialize)]
pub struct DependencyFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
}

impl ModuleGroupResult<DependencyFunction> {
    /// Build a grouped structure from flat calls
    pub fn from_calls(source_module: String, calls: Vec<Call>) -> Self {
        let total_items = calls.len();

        if calls.is_empty() {
            return ModuleGroupResult {
                module_pattern: source_module,
                function_pattern: None,
                total_items: 0,
                items: vec![],
            };
        }

        // Group by callee_module -> callee_function -> callers
        let mut by_module: HashMap<String, HashMap<(String, i64), Vec<Call>>> = HashMap::new();
        for call in calls {
            by_module
                .entry(call.callee.module.clone())
                .or_default()
                .entry((call.callee.name.clone(), call.callee.arity))
                .or_default()
                .push(call);
        }

        let mut items: Vec<ModuleGroup<DependencyFunction>> = vec![];
        for (module_name, functions_map) in by_module {
            let mut entries: Vec<DependencyFunction> = vec![];
            for ((func_name, arity), callers) in functions_map {
                entries.push(DependencyFunction {
                    name: func_name,
                    arity,
                    callers,
                });
            }

            // Sort functions by name, arity
            entries.sort_by(|a, b| (&a.name, a.arity).cmp(&(&b.name, b.arity)));

            items.push(ModuleGroup {
                name: module_name,
                file: String::new(),
                entries,
            });
        }

        // Sort modules by name
        items.sort_by(|a, b| a.name.cmp(&b.name));

        ModuleGroupResult {
            module_pattern: source_module,
            function_pattern: None,
            total_items,
            items,
        }
    }
}

impl Execute for DependsOnCmd {
    type Output = ModuleGroupResult<DependencyFunction>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependencies(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(<ModuleGroupResult<DependencyFunction>>::from_calls(self.module, calls))
    }
}
