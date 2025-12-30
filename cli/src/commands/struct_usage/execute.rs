use std::collections::{BTreeMap, HashSet};
use std::error::Error;

use serde::Serialize;

use super::StructUsageCmd;
use crate::commands::Execute;
use db::queries::struct_usage::{find_struct_usage, StructUsageEntry};
use db::types::ModuleGroupResult;

/// A function that uses a struct type
#[derive(Debug, Clone, Serialize)]
pub struct UsageInfo {
    pub name: String,
    pub arity: i64,
    pub inputs: String,
    pub returns: String,
    pub line: i64,
}

/// A module and its usage counts for a struct type
#[derive(Debug, Clone, Serialize)]
pub struct ModuleStructUsage {
    pub name: String,
    pub accepts_count: i64,
    pub returns_count: i64,
    pub total: i64,
}

/// Result containing aggregated module-level struct usage
#[derive(Debug, Clone, Serialize)]
pub struct StructModulesResult {
    pub struct_pattern: String,
    pub total_modules: usize,
    pub total_functions: usize,
    pub modules: Vec<ModuleStructUsage>,
}

/// Output type that can be either detailed or aggregated
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum StructUsageOutput {
    Detailed(ModuleGroupResult<UsageInfo>),
    ByModule(StructModulesResult),
}

/// Build grouped result from flat StructUsageEntry list
fn build_usage_info_result(
    pattern: String,
    module_filter: Option<String>,
    entries: Vec<StructUsageEntry>,
) -> ModuleGroupResult<UsageInfo> {
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

/// Build aggregated result from flat StructUsageEntry list
fn build_struct_modules_result(pattern: String, entries: Vec<StructUsageEntry>) -> StructModulesResult {
    // Aggregate by module, tracking which functions accept vs return
    let mut module_map: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    let mut module_accepts: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    let mut module_returns: BTreeMap<String, HashSet<String>> = BTreeMap::new();

    for entry in &entries {
        // Track unique functions per module
        module_map
            .entry(entry.module.clone())
            .or_default()
            .insert(format!("{}/{}", entry.name, entry.arity));

        // Check if function accepts the type
        if entry.inputs_string.contains(&pattern) {
            module_accepts
                .entry(entry.module.clone())
                .or_default()
                .insert(format!("{}/{}", entry.name, entry.arity));
        }

        // Check if function returns the type
        if entry.return_string.contains(&pattern) {
            module_returns
                .entry(entry.module.clone())
                .or_default()
                .insert(format!("{}/{}", entry.name, entry.arity));
        }
    }

    // Convert to result type, sorted by total count descending
    let mut modules: Vec<ModuleStructUsage> = module_map
        .into_iter()
        .map(|(name, functions)| {
            let accepts_count = module_accepts
                .get(&name)
                .map(|s| s.len() as i64)
                .unwrap_or(0);
            let returns_count = module_returns
                .get(&name)
                .map(|s| s.len() as i64)
                .unwrap_or(0);
            let total = functions.len() as i64;

            ModuleStructUsage {
                name,
                accepts_count,
                returns_count,
                total,
            }
        })
        .collect();

    // Sort by total count descending, then by module name
    modules.sort_by(|a, b| {
        let cmp = b.total.cmp(&a.total);
        if cmp == std::cmp::Ordering::Equal {
            a.name.cmp(&b.name)
        } else {
            cmp
        }
    });

    let total_modules = modules.len();
    let total_functions = entries.len();

    StructModulesResult {
        struct_pattern: pattern,
        total_modules,
        total_functions,
        modules,
    }
}

impl Execute for StructUsageCmd {
    type Output = StructUsageOutput;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let entries = find_struct_usage(
            db,
            &self.pattern,
            &self.common.project,
            self.common.regex,
            self.module.as_deref(),
            self.common.limit,
        )?;

        if self.by_module {
            Ok(StructUsageOutput::ByModule(
                build_struct_modules_result(self.pattern, entries),
            ))
        } else {
            Ok(StructUsageOutput::Detailed(
                build_usage_info_result(self.pattern, self.module, entries),
            ))
        }
    }
}
