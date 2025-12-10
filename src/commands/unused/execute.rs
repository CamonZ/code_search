use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::UnusedCmd;
use crate::commands::Execute;
use crate::queries::unused::{find_unused_functions, UnusedFunction};

/// An unused function within a module
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFunc {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
}

/// A module containing unused functions
#[derive(Debug, Clone, Serialize)]
pub struct UnusedModule {
    pub name: String,
    pub file: String,
    pub functions: Vec<UnusedFunc>,
}

/// Result of the unused command execution
#[derive(Debug, Default, Serialize)]
pub struct UnusedResult {
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_filter: Option<String>,
    pub private_only: bool,
    pub public_only: bool,
    pub exclude_generated: bool,
    pub total_unused: usize,
    pub modules: Vec<UnusedModule>,
}

impl UnusedResult {
    /// Build grouped result from flat UnusedFunction list
    fn from_functions(
        project: String,
        module_filter: Option<String>,
        private_only: bool,
        public_only: bool,
        exclude_generated: bool,
        functions: Vec<UnusedFunction>,
    ) -> Self {
        let total_unused = functions.len();

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, (String, Vec<UnusedFunc>)> = BTreeMap::new();

        for func in functions {
            let unused_func = UnusedFunc {
                name: func.name,
                arity: func.arity,
                kind: func.kind,
                line: func.line,
            };

            module_map
                .entry(func.module)
                .or_insert_with(|| (func.file, Vec::new()))
                .1
                .push(unused_func);
        }

        let modules: Vec<UnusedModule> = module_map
            .into_iter()
            .map(|(name, (file, functions))| UnusedModule {
                name,
                file,
                functions,
            })
            .collect();

        UnusedResult {
            project,
            module_filter,
            private_only,
            public_only,
            exclude_generated,
            total_unused,
            modules,
        }
    }
}

impl Execute for UnusedCmd {
    type Output = UnusedResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let functions = find_unused_functions(
            db,
            self.module.as_deref(),
            &self.project,
            self.regex,
            self.private_only,
            self.public_only,
            self.exclude_generated,
            self.limit,
        )?;

        Ok(UnusedResult::from_functions(
            self.project,
            self.module,
            self.private_only,
            self.public_only,
            self.exclude_generated,
            functions,
        ))
    }
}

