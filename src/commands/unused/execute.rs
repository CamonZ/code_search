use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::UnusedCmd;
use crate::commands::Execute;
use crate::queries::unused::{find_unused_functions, UnusedFunction};
use crate::types::{ModuleCollectionResult, ModuleGroup};

/// An unused function within a module
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFunc {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
}

impl ModuleCollectionResult<UnusedFunc> {
    /// Build grouped result from flat UnusedFunction list
    fn from_functions(
        module_pattern: String,
        functions: Vec<UnusedFunction>,
    ) -> Self {
        let total_items = functions.len();

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

        let items: Vec<ModuleGroup<UnusedFunc>> = module_map
            .into_iter()
            .map(|(name, (file, entries))| ModuleGroup { name, file, entries })
            .collect();

        ModuleCollectionResult {
            module_pattern,
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items,
            items,
        }
    }
}

impl Execute for UnusedCmd {
    type Output = ModuleCollectionResult<UnusedFunc>;

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

        Ok(<ModuleCollectionResult<UnusedFunc>>::from_functions(
            self.module.unwrap_or_else(|| "*".to_string()),
            functions,
        ))
    }
}

