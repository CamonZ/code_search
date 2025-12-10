use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::queries::depends_on::{find_dependencies, DependencyCall};

/// A caller function that calls into a dependency
#[derive(Debug, Clone, Serialize)]
pub struct DependencyCaller {
    pub module: String,
    pub function: String,
    pub arity: i64,
    pub kind: String,
    pub start_line: i64,
    pub end_line: i64,
    pub file: String,
    pub line: i64,
}

/// A function in a dependency module being called
#[derive(Debug, Clone, Serialize)]
pub struct DependencyFunction {
    pub name: String,
    pub arity: i64,
    pub callers: Vec<DependencyCaller>,
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
    /// Build a grouped structure from flat dependency calls
    pub fn from_calls(source_module: String, calls: Vec<DependencyCall>) -> Self {
        let total_calls = calls.len();

        if calls.is_empty() {
            return DependsOnResult {
                source_module,
                total_calls: 0,
                modules: vec![],
            };
        }

        // Group by callee_module -> callee_function -> callers
        let mut by_module: HashMap<String, HashMap<(String, i64), Vec<&DependencyCall>>> = HashMap::new();
        for call in &calls {
            by_module
                .entry(call.callee_module.clone())
                .or_default()
                .entry((call.callee_function.clone(), call.callee_arity))
                .or_default()
                .push(call);
        }

        let mut modules: Vec<DependencyModule> = vec![];
        for (module_name, functions_map) in by_module {
            let mut functions: Vec<DependencyFunction> = vec![];
            for ((func_name, arity), callers) in functions_map {
                let caller_list: Vec<DependencyCaller> = callers
                    .iter()
                    .map(|c| DependencyCaller {
                        module: c.caller_module.clone(),
                        function: c.caller_function.clone(),
                        arity: c.caller_arity,
                        kind: c.caller_kind.clone(),
                        start_line: c.caller_start_line,
                        end_line: c.caller_end_line,
                        file: c.file.clone(),
                        line: c.line,
                    })
                    .collect();

                functions.push(DependencyFunction {
                    name: func_name,
                    arity,
                    callers: caller_list,
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
