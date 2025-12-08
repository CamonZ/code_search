use std::error::Error;

use serde::Serialize;

use super::TraceCmd;
use crate::commands::Execute;
use crate::queries::trace::{trace_calls, TraceStep};

/// Result of the trace command execution
#[derive(Debug, Default, Serialize)]
pub struct TraceResult {
    pub start_module: String,
    pub start_function: String,
    pub max_depth: u32,
    pub steps: Vec<TraceStep>,
}

impl Execute for TraceCmd {
    type Output = TraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = TraceResult {
            start_module: self.module.clone(),
            start_function: self.function.clone(),
            max_depth: self.depth,
            ..Default::default()
        };

        result.steps = trace_calls(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.depth,
            self.limit,
        )?;

        Ok(result)
    }
}
