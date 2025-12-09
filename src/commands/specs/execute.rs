use std::error::Error;

use serde::Serialize;

use super::SpecsCmd;
use crate::commands::Execute;
use crate::queries::specs::{find_specs, SpecDef};

/// Result of the specs command execution
#[derive(Debug, Default, Serialize)]
pub struct SpecsResult {
    pub module_pattern: String,
    pub function_pattern: Option<String>,
    pub kind_filter: Option<String>,
    pub specs: Vec<SpecDef>,
}

impl Execute for SpecsCmd {
    type Output = SpecsResult;

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

        Ok(SpecsResult {
            module_pattern: self.module,
            function_pattern: self.function,
            kind_filter: self.kind,
            specs,
        })
    }
}
