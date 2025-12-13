use std::error::Error;

use serde::Serialize;

use super::StructUsageCmd;
use crate::commands::Execute;
use crate::queries::struct_usage::{find_struct_usage, StructUsageEntry};
use crate::types::ModuleGroupResult;

/// A function that uses a struct type
#[derive(Debug, Clone, Serialize)]
pub struct UsageInfo {
    pub name: String,
    pub arity: i64,
    pub inputs: String,
    pub returns: String,
    pub line: i64,
}

impl ModuleGroupResult<UsageInfo> {
    /// Build grouped result from flat StructUsageEntry list
    fn from_entries(
        pattern: String,
        module_filter: Option<String>,
        entries: Vec<StructUsageEntry>,
    ) -> Self {
        let total_items = entries.len();

        // Use helper to group by module
        let items = crate::utils::group_by_module(entries, |entry| {
            let usage_info = UsageInfo {
                name: entry.name,
                arity: entry.arity,
                inputs: entry.inputs_string,
                returns: entry.return_string,
                line: entry.line,
            };
            (entry.module, usage_info)
        });

        ModuleGroupResult {
            module_pattern: module_filter.unwrap_or_else(|| "*".to_string()),
            function_pattern: Some(pattern),
            total_items,
            items,
        }
    }
}

impl Execute for StructUsageCmd {
    type Output = ModuleGroupResult<UsageInfo>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let entries = find_struct_usage(
            db,
            &self.pattern,
            &self.common.project,
            self.common.regex,
            self.module.as_deref(),
            self.common.limit,
        )?;

        Ok(<ModuleGroupResult<UsageInfo>>::from_entries(
            self.pattern,
            self.module,
            entries,
        ))
    }
}
