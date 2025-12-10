use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::queries::depends_on::find_dependencies;
use crate::types::Call;

/// A function in a dependency module being called
#[derive(Debug, Clone, Serialize)]
pub struct DependencyFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<Call>,
}

/// A dependency module with the functions being called
#[derive(Debug, Clone, Serialize)]
pub struct DependencyModule {
    pub name: String,
    pub functions: Vec<DependencyFunction>,
}

/// Result of the depends-on command execution
#[derive(Debug, Default, Serialize)]
pub struct DependsOnResult {
    pub source_module: String,
    pub total_calls: usize,
    pub modules: Vec<DependencyModule>,
}

impl DependsOnResult {
    /// Build a grouped structure from flat calls
    pub fn from_calls(source_module: String, calls: Vec<Call>) -> Self {
        let total_calls = calls.len();

        if calls.is_empty() {
            return DependsOnResult {
                source_module,
                total_calls: 0,
                modules: vec![],
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

        let mut modules: Vec<DependencyModule> = vec![];
        for (module_name, functions_map) in by_module {
            let mut functions: Vec<DependencyFunction> = vec![];
            for ((func_name, arity), callers) in functions_map {
                functions.push(DependencyFunction {
                    name: func_name,
                    arity,
                    callers,
                });
            }

            // Sort functions by name, arity
            functions.sort_by(|a, b| (&a.name, a.arity).cmp(&(&b.name, b.arity)));

            modules.push(DependencyModule {
                name: module_name,
                functions,
            });
        }

        // Sort modules by name
        modules.sort_by(|a, b| a.name.cmp(&b.name));

        DependsOnResult {
            source_module,
            total_calls,
            modules,
        }
    }
}

impl Execute for DependsOnCmd {
    type Output = DependsOnResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependencies(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(DependsOnResult::from_calls(self.module, calls))
    }
}
