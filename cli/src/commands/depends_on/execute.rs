use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use db::queries::depends_on::find_dependencies;
use db::types::{Call, ModuleGroupResult};
use crate::utils::convert_to_module_groups;

/// A function in a dependency module being called
#[derive(Debug, Clone, Serialize)]
pub struct DependencyFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
}

/// Build a grouped structure from flat calls
fn build_dependency_result(source_module: String, calls: Vec<Call>) -> ModuleGroupResult<DependencyFunction> {
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
            .entry(call.callee.module.to_string())
            .or_default()
            .entry((call.callee.name.to_string(), call.callee.arity))
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

impl Execute for DependsOnCmd {
    type Output = ModuleGroupResult<DependencyFunction>;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependencies(
            db,
            &self.module,
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(build_dependency_result(self.module, calls))
    }
}
