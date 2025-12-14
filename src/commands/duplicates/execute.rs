use std::error::Error;

use serde::Serialize;

use super::DuplicatesCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::duplicates::find_duplicates;

/// Result structure for duplicates command - grouped by hash
#[derive(Debug, Clone, Serialize)]
pub struct DuplicatesResult {
    pub total_groups: usize,
    pub total_duplicates: usize,
    pub groups: Vec<DuplicateGroup>,
}

/// A group of functions with the same hash
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub functions: Vec<DuplicateFunctionEntry>,
}

/// A function within a duplicate group
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateFunctionEntry {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub file: String,
}

impl Execute for DuplicatesCmd {
    type Output = DuplicatesResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let functions = find_duplicates(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
            self.exact,
        )?;

        // Group by hash
        let mut groups_map: std::collections::BTreeMap<String, Vec<DuplicateFunctionEntry>> =
            std::collections::BTreeMap::new();

        for func in functions {
            let entry = DuplicateFunctionEntry {
                module: func.module,
                name: func.name,
                arity: func.arity,
                line: func.line,
                file: func.file,
            };
            groups_map.entry(func.hash).or_insert_with(Vec::new).push(entry);
        }

        // Convert to result format
        let total_duplicates = groups_map.values().map(|v| v.len()).sum();
        let groups = groups_map
            .into_iter()
            .map(|(hash, functions)| DuplicateGroup { hash, functions })
            .collect::<Vec<_>>();
        let total_groups = groups.len();

        Ok(DuplicatesResult {
            total_groups,
            total_duplicates,
            groups,
        })
    }
}
