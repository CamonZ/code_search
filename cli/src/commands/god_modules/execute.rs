use std::error::Error;

use serde::Serialize;

use super::GodModulesCmd;
use crate::commands::Execute;
use db::queries::hotspots::{get_function_counts, get_module_connectivity, get_module_loc};
use db::types::{ModuleCollectionResult, ModuleGroup};

/// A single god module entry
#[derive(Debug, Clone, Serialize)]
pub struct GodModuleEntry {
    pub function_count: i64,
    pub loc: i64,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
}

impl Execute for GodModulesCmd {
    type Output = ModuleCollectionResult<GodModuleEntry>;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        // Get function counts for all modules
        let func_counts = get_function_counts(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
        )?;

        // Get lines of code per module
        let module_loc = get_module_loc(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
        )?;

        // Get module-level connectivity (aggregated at database level)
        let module_connectivity = get_module_connectivity(
            db,
            &self.common.project,
            self.module.as_deref(),
            self.common.regex,
        )?;

        // Build god modules: filter by thresholds and sort by total connectivity
        // Tuple: (module_name, func_count, loc, incoming, outgoing)
        let mut god_modules: Vec<(String, i64, i64, i64, i64)> = Vec::new();

        for (module_name, func_count) in func_counts {
            // Apply function count threshold
            if func_count < self.min_functions {
                continue;
            }

            // Get LoC for this module
            let loc = module_loc.get(&module_name).copied().unwrap_or(0);

            // Apply LoC threshold if set
            if loc < self.min_loc {
                continue;
            }

            // Get connectivity for this module
            let (incoming, outgoing) = module_connectivity
                .get(&module_name)
                .copied()
                .unwrap_or((0, 0));
            let total = incoming + outgoing;

            // Apply total connectivity threshold
            if total < self.min_total {
                continue;
            }

            god_modules.push((module_name, func_count, loc, incoming, outgoing));
        }

        // Sort by total connectivity (descending)
        god_modules.sort_by(|a, b| {
            let total_a = a.3 + a.4;
            let total_b = b.3 + b.4;
            total_b.cmp(&total_a)
        });

        // Apply limit
        let limit = self.common.limit as usize;
        god_modules.truncate(limit);

        // Convert to ModuleGroup entries
        let total_items = god_modules.len();
        let items: Vec<ModuleGroup<GodModuleEntry>> = god_modules
            .into_iter()
            .map(|(module_name, func_count, loc, incoming, outgoing)| {
                let total = incoming + outgoing;
                ModuleGroup {
                    name: module_name,
                    file: String::new(),
                    entries: vec![GodModuleEntry {
                        function_count: func_count,
                        loc,
                        incoming,
                        outgoing,
                        total,
                    }],
                    function_count: Some(func_count),
                }
            })
            .collect();

        Ok(ModuleCollectionResult {
            module_pattern: self.module.clone().unwrap_or_else(|| "*".to_string()),
            function_pattern: None,
            kind_filter: Some("god".to_string()),
            name_filter: None,
            total_items,
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_god_modules_cmd_structure() {
        // Test that GodModulesCmd is created correctly
        let cmd = GodModulesCmd {
            min_functions: 30,
            min_loc: 500,
            min_total: 15,
            module: Some("MyApp".to_string()),
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 20,
            },
        };

        assert_eq!(cmd.min_functions, 30);
        assert_eq!(cmd.min_loc, 500);
        assert_eq!(cmd.min_total, 15);
        assert_eq!(cmd.module, Some("MyApp".to_string()));
    }
}
