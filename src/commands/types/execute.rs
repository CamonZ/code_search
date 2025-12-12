use std::error::Error;

use serde::Serialize;

use super::TypesCmd;
use crate::commands::Execute;
use crate::queries::types::{find_types, TypeInfo};
use crate::types::ModuleCollectionResult;

/// A single type definition
#[derive(Debug, Clone, Serialize)]
pub struct TypeEntry {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub params: String,
    pub line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub definition: String,
}

impl ModuleCollectionResult<TypeEntry> {
    /// Build grouped result from flat TypeInfo list
    fn from_types(
        module_pattern: String,
        name_filter: Option<String>,
        kind_filter: Option<String>,
        types: Vec<TypeInfo>,
    ) -> Self {
        let total_items = types.len();

        // Use helper to group by module
        let items = crate::utils::group_by_module(types, |type_info| {
            let entry = TypeEntry {
                name: type_info.name,
                kind: type_info.kind,
                params: type_info.params,
                line: type_info.line,
                definition: type_info.definition,
            };
            // File is intentionally empty for types because the call graph data model
            // does not track file locations for @type definitions (only for functions).
            (type_info.module, entry)
        });

        ModuleCollectionResult {
            module_pattern,
            function_pattern: None,
            name_filter,
            kind_filter,
            total_items,
            items,
        }
    }
}

impl Execute for TypesCmd {
    type Output = ModuleCollectionResult<TypeEntry>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let types = find_types(
            db,
            &self.module,
            self.name.as_deref(),
            self.kind.as_deref(),
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(ModuleCollectionResult::from_types(
            self.module,
            self.name,
            self.kind,
            types,
        ))
    }
}
