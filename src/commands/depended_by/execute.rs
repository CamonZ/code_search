use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::DependedByCmd;
use crate::commands::Execute;
use crate::queries::depended_by::find_dependents;
use crate::types::Call;

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

/// A dependent module with caller functions
#[derive(Debug, Clone, Serialize)]
pub struct DependentModule {
    pub name: String,
    pub callers: Vec<DependentCaller>,
}

/// Result of the depended-by command execution
#[derive(Debug, Default, Serialize)]
pub struct DependedByResult {
    pub target_module: String,
    pub total_calls: usize,
    pub modules: Vec<DependentModule>,
}

impl DependedByResult {
    /// Build a grouped structure from flat calls
    pub fn from_calls(target_module: String, calls: Vec<Call>) -> Self {
        let total_calls = calls.len();

        if calls.is_empty() {
            return DependedByResult {
                target_module,
                total_calls: 0,
                modules: vec![],
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

        let mut modules: Vec<DependentModule> = vec![];
        for (module_name, callers_map) in by_module {
            let mut callers: Vec<DependentCaller> = vec![];
            for ((func_name, arity), func_calls) in callers_map {
                let first = func_calls[0];
                let targets: Vec<DependentTarget> = func_calls
                    .iter()
                    .map(|c| DependentTarget {
                        function: c.callee.name.clone(),
                        arity: c.callee.arity,
                        line: c.line,
                    })
                    .collect();

                callers.push(DependentCaller {
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
            callers.sort_by(|a, b| (&a.function, a.arity).cmp(&(&b.function, b.arity)));

            modules.push(DependentModule {
                name: module_name,
                callers,
            });
        }

        // Sort modules by name
        modules.sort_by(|a, b| a.name.cmp(&b.name));

        DependedByResult {
            target_module,
            total_calls,
            modules,
        }
    }
}

impl Execute for DependedByCmd {
    type Output = DependedByResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let calls = find_dependents(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(DependedByResult::from_calls(self.module, calls))
    }
}
