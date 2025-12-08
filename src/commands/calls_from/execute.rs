use std::error::Error;

use serde::Serialize;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::queries::calls_from::{find_calls_from, CallEdge};

/// Result of the calls-from command execution
#[derive(Debug, Default, Serialize)]
pub struct CallsFromResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub calls: Vec<CallEdge>,
}

impl Execute for CallsFromCmd {
    type Output = CallsFromResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = CallsFromResult {
            module_pattern: self.module.clone(),
            function_pattern: self.function.clone().unwrap_or_default(),
            ..Default::default()
        };

        result.calls = find_calls_from(
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
