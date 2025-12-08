use std::error::Error;

use serde::Serialize;

use super::UnusedCmd;
use crate::commands::Execute;
use crate::queries::unused::{find_unused_functions, UnusedFunction};

/// Result of the unused command execution
#[derive(Debug, Default, Serialize)]
pub struct UnusedResult {
    pub project: String,
    pub module_filter: Option<String>,
    pub private_only: bool,
    pub public_only: bool,
    pub exclude_generated: bool,
    pub functions: Vec<UnusedFunction>,
}

impl Execute for UnusedCmd {
    type Output = UnusedResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = UnusedResult {
            project: self.project.clone(),
            module_filter: self.module.clone(),
            private_only: self.private_only,
            public_only: self.public_only,
            exclude_generated: self.exclude_generated,
            ..Default::default()
        };

        result.functions = find_unused_functions(
            db,
            self.module.as_deref(),
            &self.project,
            self.regex,
            self.private_only,
            self.public_only,
            self.exclude_generated,
            self.limit,
        )?;

        Ok(result)
    }
}

