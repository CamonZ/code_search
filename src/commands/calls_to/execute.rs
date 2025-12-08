use std::error::Error;

use serde::Serialize;

use super::CallsToCmd;
use crate::commands::Execute;
use crate::queries::calls_to::{find_calls_to, CallEdge};

/// Result of the calls-to command execution
#[derive(Debug, Default, Serialize)]
pub struct CallsToResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub calls: Vec<CallEdge>,
}

impl Execute for CallsToCmd {
    type Output = CallsToResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = CallsToResult {
            module_pattern: self.module.clone(),
            function_pattern: self.function.clone().unwrap_or_default(),
            ..Default::default()
        };

        result.calls = find_calls_to(
            db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
