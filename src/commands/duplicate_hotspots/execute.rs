use std::error::Error;

use serde::Serialize;

use super::DuplicateHotspotsCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::duplicates::find_duplicates;

/// Result structure for duplicate-hotspots command - ranked by module
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateHotspotsResult {
    pub total_modules: usize,
    pub total_duplicates: i64,
    pub modules: Vec<ModuleDuplicates>,
}

/// A module with its duplicate functions
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDuplicates {
    pub name: String,
    pub duplicate_count: i64,
    pub top_duplicates: Vec<DuplicateSummary>,
}

/// Summary of a duplicated function
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateSummary {
    pub name: String,
    pub arity: i64,
    pub copy_count: i64,
}

impl Execute for DuplicateHotspotsCmd {
    type Output = DuplicateHotspotsResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let functions = find_duplicates(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
            self.exact,
        )?;

        // Group by module first
        let mut module_map: std::collections::BTreeMap<
            String,
            Vec<(String, String, i64)>,
        > = std::collections::BTreeMap::new();

        for func in functions {
            let module = func.module.clone();
            module_map
                .entry(module)
                .or_insert_with(Vec::new)
                .push((func.hash, func.name, func.arity));
        }

        // Aggregate by module and find top duplicates per module
        let mut modules = Vec::new();
        for (module_name, funcs) in module_map {
            // Group by function name/arity to count copies
            let mut func_map: std::collections::BTreeMap<
                (String, i64),
                i64,
            > = std::collections::BTreeMap::new();

            for (_hash, name, arity) in &funcs {
                let key = (name.clone(), *arity);
                *func_map.entry(key).or_insert(0) += 1;
            }

            // Convert to DuplicateSummary and sort by copy count (descending)
            let mut summaries = func_map
                .into_iter()
                .map(|((name, arity), count)| DuplicateSummary {
                    name,
                    arity,
                    copy_count: count,
                })
                .collect::<Vec<_>>();

            summaries.sort_by(|a, b| b.copy_count.cmp(&a.copy_count));

            let duplicate_count = summaries.len() as i64;
            modules.push(ModuleDuplicates {
                name: module_name,
                duplicate_count,
                top_duplicates: summaries,
            });
        }

        // Sort modules by duplicate count (descending)
        modules.sort_by(|a, b| b.duplicate_count.cmp(&a.duplicate_count));

        let total_duplicates: i64 = modules.iter().map(|m| m.duplicate_count).sum();
        let total_modules = modules.len();

        Ok(DuplicateHotspotsResult {
            total_modules,
            total_duplicates,
            modules,
        })
    }
}
