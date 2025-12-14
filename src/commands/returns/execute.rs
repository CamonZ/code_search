use std::error::Error;

use serde::Serialize;

use super::ReturnsCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::returns::{find_returns, ReturnEntry};
use crate::types::ModuleGroupResult;

/// A function's return type information
#[derive(Debug, Clone, Serialize)]
pub struct ReturnInfo {
    pub name: String,
    pub arity: i64,
    pub return_type: String,
    pub line: i64,
}

impl ModuleGroupResult<ReturnInfo> {
    /// Build grouped result from flat ReturnEntry list
    fn from_entries(
        pattern: String,
        module_filter: Option<String>,
        entries: Vec<ReturnEntry>,
    ) -> Self {
        let total_items = entries.len();

        // Use helper to group by module
        let items = crate::utils::group_by_module(entries, |entry| {
            let return_info = ReturnInfo {
                name: entry.name,
                arity: entry.arity,
                return_type: entry.return_string,
                line: entry.line,
            };
            (entry.module, return_info)
        });

        ModuleGroupResult {
            module_pattern: module_filter.unwrap_or_else(|| "*".to_string()),
            function_pattern: Some(pattern),
            total_items,
            items,
        }
    }
}

impl Execute for ReturnsCmd {
    type Output = ModuleGroupResult<ReturnInfo>;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let entries = find_returns(
            db,
            &self.pattern,
            &self.common.project,
            self.common.regex,
            self.module.as_deref(),
            self.common.limit,
        )?;

        Ok(<ModuleGroupResult<ReturnInfo>>::from_entries(
            self.pattern,
            self.module,
            entries,
        ))
    }
}
