use std::error::Error;

use serde::Serialize;

use super::StructCmd;
use crate::commands::Execute;
use crate::queries::structs::{find_struct_fields, group_fields_into_structs, StructDefinition};

/// Result of the struct command execution
#[derive(Debug, Default, Serialize)]
pub struct StructResult {
    pub module_pattern: String,
    pub structs: Vec<StructDefinition>,
}

impl Execute for StructCmd {
    type Output = StructResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = StructResult {
            module_pattern: self.module.clone(),
            ..Default::default()
        };

        let fields = find_struct_fields(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        // Group fields by (project, module)
        result.structs = group_fields_into_structs(fields);

        Ok(result)
    }
}

