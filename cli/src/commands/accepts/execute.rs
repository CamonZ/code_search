use std::error::Error;

use serde::Serialize;

use super::AcceptsCmd;
use crate::commands::Execute;
use db::queries::accepts::{find_accepts, AcceptsEntry};
use db::types::ModuleGroupResult;

/// A function's input type information
#[derive(Debug, Clone, Serialize)]
pub struct AcceptsInfo {
    pub name: String,
    pub arity: i64,
    pub inputs: String,
    pub return_type: String,
    pub line: i64,
}

fn build_accepts_result(
    pattern: String,
    module_filter: Option<String>,
    entries: Vec<AcceptsEntry>,
) -> ModuleGroupResult<AcceptsInfo> {
    let total_items = entries.len();

    // Use helper to group by module
    let items = crate::utils::group_by_module(entries, |entry| {
        let accepts_info = AcceptsInfo {
            name: entry.name,
            arity: entry.arity,
            inputs: entry.inputs_string,
            return_type: entry.return_string,
            line: entry.line,
        };
        (entry.module, accepts_info)
    });

    ModuleGroupResult {
        module_pattern: module_filter.unwrap_or_else(|| "*".to_string()),
        function_pattern: Some(pattern),
        total_items,
        items,
    }
}

impl Execute for AcceptsCmd {
    type Output = ModuleGroupResult<AcceptsInfo>;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let entries = find_accepts(
            db,
            &self.pattern,
            &self.common.project,
            self.common.regex,
            self.module.as_deref(),
            self.common.limit,
        )?;

        Ok(build_accepts_result(
            self.pattern,
            self.module,
            entries,
        ))
    }
}
