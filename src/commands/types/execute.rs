use std::error::Error;

use serde::Serialize;

use super::TypesCmd;
use crate::commands::Execute;
use crate::queries::types::{find_types, TypeInfo};

/// Result of the types command execution
#[derive(Debug, Default, Serialize)]
pub struct TypesResult {
    pub module_pattern: String,
    pub name_filter: Option<String>,
    pub kind_filter: Option<String>,
    pub types: Vec<TypeInfo>,
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

        Ok(TypesResult {
            module_pattern: self.module,
            name_filter: self.name,
            kind_filter: self.kind,
            types,
        })
    }
}
