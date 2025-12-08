use std::error::Error;

use serde::Serialize;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::queries::depends_on::{find_dependencies, ModuleDependency};

/// Result of the depends-on command execution
#[derive(Debug, Default, Serialize)]
pub struct DependsOnResult {
    pub source_module: String,
    pub dependencies: Vec<ModuleDependency>,
}

impl Execute for DependsOnCmd {
    type Output = DependsOnResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = DependsOnResult {
            source_module: self.module.clone(),
            ..Default::default()
        };

        result.dependencies = find_dependencies(
            db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
