use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::queries::depends_on::find_dependencies;
use crate::types::{Call, ModuleGroupResult};
use crate::utils::convert_to_module_groups;

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
        // Using BTreeMap for automatic sorting
        let mut by_module: BTreeMap<String, BTreeMap<(String, i64), Vec<Call>>> = BTreeMap::new();
        for call in calls {
            by_module
                .entry(call.callee.module.clone())
                .or_default()
                .entry((call.callee.name.clone(), call.callee.arity))
                .or_default()
                .push(call);
        }

        // Convert to ModuleGroup structure
        let items = convert_to_module_groups(
            by_module,
            |(func_name, arity), callers| DependencyFunction {
                name: func_name,
                arity,
                callers,
            },
            // File is intentionally empty because dependencies are the grouping key,
            // and a module can depend on functions defined across multiple files.
            // The dependency targets themselves carry file information where needed.
            |_module, _map| String::new(),
        );

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
