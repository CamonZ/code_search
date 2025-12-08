use std::error::Error;

use serde::Serialize;

use super::ReverseTraceCmd;
use crate::commands::Execute;
use crate::queries::reverse_trace::{reverse_trace_calls, ReverseTraceStep};

/// Result of the reverse-trace command execution
#[derive(Debug, Default, Serialize)]
pub struct ReverseTraceResult {
    pub target_module: String,
    pub target_function: String,
    pub max_depth: u32,
    pub steps: Vec<ReverseTraceStep>,
}

impl Execute for ReverseTraceCmd {
    type Output = ReverseTraceResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = ReverseTraceResult {
            target_module: self.module.clone(),
            target_function: self.function.clone(),
            max_depth: self.depth,
            ..Default::default()
        };

        result.steps = reverse_trace_calls(
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
