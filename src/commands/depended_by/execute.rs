use std::error::Error;

use serde::Serialize;

use super::DependedByCmd;
use crate::commands::Execute;
use crate::queries::depended_by::{find_dependents, ModuleDependent};

/// Result of the depended-by command execution
#[derive(Debug, Default, Serialize)]
pub struct DependedByResult {
    pub target_module: String,
    pub dependents: Vec<ModuleDependent>,
}

impl Execute for DependedByCmd {
    type Output = DependedByResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = DependedByResult {
            target_module: self.module.clone(),
            ..Default::default()
        };

        result.dependents = find_dependents(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
