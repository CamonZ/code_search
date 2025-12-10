use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::TypesCmd;
use crate::commands::Execute;
use crate::queries::types::{find_types, TypeInfo};

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

/// A module containing type definitions
#[derive(Debug, Clone, Serialize)]
pub struct TypeModule {
    pub name: String,
    pub types: Vec<TypeEntry>,
}

/// Result of the types command execution
#[derive(Debug, Default, Serialize)]
pub struct TypesResult {
    pub module_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_filter: Option<String>,
    pub total_types: usize,
    pub modules: Vec<TypeModule>,
}

impl TypesResult {
    /// Build grouped result from flat TypeInfo list
    fn from_types(
        module_pattern: String,
        name_filter: Option<String>,
        kind_filter: Option<String>,
        types: Vec<TypeInfo>,
    ) -> Self {
        let total_types = types.len();

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<TypeEntry>> = BTreeMap::new();

        for type_info in types {
            let entry = TypeEntry {
                name: type_info.name,
                kind: type_info.kind,
                params: type_info.params,
                line: type_info.line,
                definition: type_info.definition,
            };

            module_map.entry(type_info.module).or_default().push(entry);
        }

        let modules: Vec<TypeModule> = module_map
            .into_iter()
            .map(|(name, types)| TypeModule { name, types })
            .collect();

        TypesResult {
            module_pattern,
            name_filter,
            kind_filter,
            total_types,
            modules,
        }
    }
}

impl Execute for TypesCmd {
    type Output = TypesResult;

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

        Ok(TypesResult::from_types(
            self.module,
            self.name,
            self.kind,
            types,
        ))
    }
}
