use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::DuplicatesCmd;
use crate::commands::Execute;
use db::queries::duplicates::find_duplicates;

// =============================================================================
// Detailed mode types (default)
// =============================================================================

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

// =============================================================================
// ByModule mode types
// =============================================================================

/// Result structure for --by-module mode - ranked by module
#[derive(Debug, Clone, Serialize)]
pub struct DuplicatesByModuleResult {
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

// =============================================================================
// Output enum
// =============================================================================

/// Output type that can be either detailed or aggregated by module
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DuplicatesOutput {
    Detailed(DuplicatesResult),
    ByModule(DuplicatesByModuleResult),
}

// =============================================================================
// Execute implementation
// =============================================================================

impl Execute for DuplicatesCmd {
    type Output = DuplicatesOutput;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let functions = find_duplicates(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
            self.exact,
            self.exclude_generated,
        )?;

        if self.by_module {
            Ok(DuplicatesOutput::ByModule(build_by_module_result(functions)))
        } else {
            Ok(DuplicatesOutput::Detailed(build_detailed_result(functions)))
        }
    }
}

fn build_detailed_result(
    functions: Vec<db::queries::duplicates::DuplicateFunction>,
) -> DuplicatesResult {
    // Group by hash
    let mut groups_map: BTreeMap<String, Vec<DuplicateFunctionEntry>> = BTreeMap::new();

    for func in functions {
        let entry = DuplicateFunctionEntry {
            module: func.module,
            name: func.name,
            arity: func.arity,
            line: func.line,
            file: func.file,
        };
        groups_map.entry(func.hash).or_default().push(entry);
    }

    // Convert to result format
    let total_duplicates = groups_map.values().map(|v| v.len()).sum();
    let groups = groups_map
        .into_iter()
        .map(|(hash, functions)| DuplicateGroup { hash, functions })
        .collect::<Vec<_>>();
    let total_groups = groups.len();

    DuplicatesResult {
        total_groups,
        total_duplicates,
        groups,
    }
}

fn build_by_module_result(
    functions: Vec<db::queries::duplicates::DuplicateFunction>,
) -> DuplicatesByModuleResult {
    // Group by module first
    let mut module_map: BTreeMap<String, Vec<(String, String, i64)>> = BTreeMap::new();

    for func in functions {
        module_map
            .entry(func.module)
            .or_default()
            .push((func.hash, func.name, func.arity));
    }

    // Aggregate by module and find top duplicates per module
    let mut modules = Vec::new();
    for (module_name, funcs) in module_map {
        // Group by function name/arity to count copies
        let mut func_map: BTreeMap<(String, i64), i64> = BTreeMap::new();

        for (_hash, name, arity) in &funcs {
            let key = (name.clone(), *arity);
            *func_map.entry(key).or_insert(0) += 1;
        }

        // Convert to DuplicateSummary and sort by copy count (descending)
        let mut summaries: Vec<DuplicateSummary> = func_map
            .into_iter()
            .map(|((name, arity), count)| DuplicateSummary {
                name,
                arity,
                copy_count: count,
            })
            .collect();

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

    DuplicatesByModuleResult {
        total_modules,
        total_duplicates,
        modules,
    }
}
