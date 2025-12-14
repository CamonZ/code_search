use std::error::Error;

use serde::Serialize;

use super::StructModulesCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;

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

impl Execute for StructModulesCmd {
    type Output = StructModulesResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        // Reuse the struct_usage query
        let entries = crate::queries::struct_usage::find_struct_usage(
            db,
            &self.pattern,
            &self.common.project,
            self.common.regex,
            self.module.as_deref(),
            self.common.limit,
        )?;

        // Aggregate by module, tracking which functions accept vs return
        let mut module_map: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
            std::collections::BTreeMap::new();
        let mut module_accepts: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
            std::collections::BTreeMap::new();
        let mut module_returns: std::collections::BTreeMap<String, std::collections::HashSet<String>> =
            std::collections::BTreeMap::new();

        for entry in &entries {
            // Track unique functions per module
            module_map
                .entry(entry.module.clone())
                .or_insert_with(std::collections::HashSet::new)
                .insert(format!("{}/{}", entry.name, entry.arity));

            // Check if function accepts the type
            if entry.inputs_string.contains(&self.pattern) {
                module_accepts
                    .entry(entry.module.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(format!("{}/{}", entry.name, entry.arity));
            }

            // Check if function returns the type
            if entry.return_string.contains(&self.pattern) {
                module_returns
                    .entry(entry.module.clone())
                    .or_insert_with(std::collections::HashSet::new)
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

        Ok(StructModulesResult {
            struct_pattern: self.pattern,
            total_modules,
            total_functions,
            modules,
        })
    }
}
