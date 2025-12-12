use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::SpecsCmd;
use crate::commands::Execute;
use crate::queries::specs::{find_specs, SpecDef};
use crate::types::{ModuleCollectionResult, ModuleGroup};

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

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<SpecEntry>> = BTreeMap::new();

        for spec in specs {
            let entry = SpecEntry {
                name: spec.name,
                arity: spec.arity,
                kind: spec.kind,
                line: spec.line,
                inputs: spec.inputs_string,
                returns: spec.return_string,
                full: spec.full,
            };

            module_map.entry(spec.module).or_default().push(entry);
        }

        let items: Vec<ModuleGroup<SpecEntry>> = module_map
            .into_iter()
            .map(|(name, entries)| ModuleGroup {
                name,
                // File is intentionally empty for specs because the call graph data model
                // does not track file locations for @spec definitions (only for functions).
                file: String::new(),
                entries,
            })
            .collect();

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
