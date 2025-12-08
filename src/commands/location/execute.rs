use std::error::Error;

use serde::Serialize;

use super::LocationCmd;
use crate::commands::Execute;
use crate::queries::location::{find_locations, FunctionLocation};

/// Result of the location command execution
#[derive(Debug, Default, Serialize)]
pub struct LocationResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub locations: Vec<FunctionLocation>,
}

impl Execute for LocationCmd {
    type Output = LocationResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = LocationResult {
            module_pattern: self.module.clone().unwrap_or_default(),
            function_pattern: self.function.clone(),
            ..Default::default()
        };

        result.locations = find_locations(
            db,
            self.module.as_deref(),
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
