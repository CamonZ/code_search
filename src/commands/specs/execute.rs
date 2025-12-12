use std::error::Error;

use serde::Serialize;

use super::SpecsCmd;
use crate::commands::Execute;
use crate::queries::specs::{find_specs, SpecDef};
use crate::types::ModuleCollectionResult;

/// A single spec definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecEntry {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inputs: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub returns: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub full: String,
}

impl ModuleCollectionResult<SpecEntry> {
    /// Build grouped result from flat SpecDef list
    fn from_specs(
        module_pattern: String,
        function_pattern: Option<String>,
        kind_filter: Option<String>,
        specs: Vec<SpecDef>,
    ) -> Self {
        let total_items = specs.len();

        // Use helper to group by module
        let items = crate::utils::group_by_module(specs, |spec| {
            let entry = SpecEntry {
                name: spec.name,
                arity: spec.arity,
                kind: spec.kind,
                line: spec.line,
                inputs: spec.inputs_string,
                returns: spec.return_string,
                full: spec.full,
            };
            // File is intentionally empty for specs because the call graph data model
            // does not track file locations for @spec definitions (only for functions).
            (spec.module, entry)
        });

        ModuleCollectionResult {
            module_pattern,
            function_pattern,
            kind_filter,
            name_filter: None,
            total_items,
            items,
        }
    }
}

impl Execute for SpecsCmd {
    type Output = ModuleCollectionResult<SpecEntry>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let specs = find_specs(
            db,
            &self.module,
            self.function.as_deref(),
            self.kind.as_deref(),
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(<ModuleCollectionResult<SpecEntry>>::from_specs(
            self.module,
            self.function,
            self.kind,
            specs,
        ))
    }
}
