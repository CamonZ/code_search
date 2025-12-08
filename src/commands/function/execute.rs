use std::error::Error;

use serde::Serialize;

use super::FunctionCmd;
use crate::commands::Execute;
use crate::queries::function::{find_functions, FunctionSignature};

/// Result of the function command execution
#[derive(Debug, Default, Serialize)]
pub struct FunctionResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub functions: Vec<FunctionSignature>,
}

impl Execute for FunctionCmd {
    type Output = FunctionResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = FunctionResult {
            module_pattern: self.module.clone(),
            function_pattern: self.function.clone(),
            ..Default::default()
        };

        result.functions = find_functions(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
