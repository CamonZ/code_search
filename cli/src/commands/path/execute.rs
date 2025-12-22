use std::error::Error;

use serde::Serialize;

use super::PathCmd;
use crate::commands::Execute;
use crate::queries::path::{find_paths, CallPath};

/// Result of the path command execution
#[derive(Debug, Default, Serialize)]
pub struct PathResult {
    pub from_module: String,
    pub from_function: String,
    pub to_module: String,
    pub to_function: String,
    pub max_depth: u32,
    pub paths: Vec<CallPath>,
}

impl Execute for PathCmd {
    type Output = PathResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = PathResult {
            from_module: self.from_module.clone(),
            from_function: self.from_function.clone(),
            to_module: self.to_module.clone(),
            to_function: self.to_function.clone(),
            max_depth: self.depth,
            ..Default::default()
        };

        result.paths = find_paths(
            db,
            &self.from_module,
            &self.from_function,
            self.from_arity,
            &self.to_module,
            &self.to_function,
            self.to_arity,
            &self.project,
            self.depth,
            self.limit,
        )?;

        Ok(result)
    }
}