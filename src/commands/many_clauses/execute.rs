use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::ManyClausesCmd;
use crate::commands::Execute;
use crate::queries::many_clauses::find_many_clauses;
use crate::types::{ModuleCollectionResult, ModuleGroup};

/// A single function with many clauses entry
#[derive(Debug, Clone, Serialize)]
pub struct ManyClausesEntry {
    pub name: String,
    pub arity: i64,
    pub clauses: i64,
    pub first_line: i64,
    pub last_line: i64,
    pub file: String,
}

impl Execute for ManyClausesCmd {
    type Output = ModuleCollectionResult<ManyClausesEntry>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let many_clauses = find_many_clauses(
            db,
            self.min_clauses,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.include_generated,
            self.common.limit,
        )?;

        let total_items = many_clauses.len();

        // Group by module while preserving sort order (most clauses first)
        let mut module_order: Vec<String> = Vec::new();
        let mut module_map: HashMap<String, (String, Vec<ManyClausesEntry>)> = HashMap::new();

        for func in many_clauses {
            let entry = ManyClausesEntry {
                name: func.name,
                arity: func.arity,
                clauses: func.clauses,
                first_line: func.first_line,
                last_line: func.last_line,
                file: func.file.clone(),
            };

            if !module_map.contains_key(&func.module) {
                module_order.push(func.module.clone());
            }

            module_map
                .entry(func.module)
                .or_insert_with(|| (func.file, Vec::new()))
                .1
                .push(entry);
        }

        let items: Vec<ModuleGroup<ManyClausesEntry>> = module_order
            .into_iter()
            .filter_map(|name| {
                module_map.remove(&name).map(|(file, entries)| ModuleGroup {
                    name,
                    file,
                    entries,
                    function_count: None,
                })
            })
            .collect();

        Ok(ModuleCollectionResult {
            module_pattern: self.module.clone().unwrap_or_else(|| "*".to_string()),
            function_pattern: None,
            kind_filter: None,
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
    fn test_many_clauses_cmd_structure() {
        let cmd = ManyClausesCmd {
            min_clauses: 10,
            include_generated: false,
            module: Some("MyApp".to_string()),
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 20,
            },
        };

        assert_eq!(cmd.min_clauses, 10);
        assert!(!cmd.include_generated);
        assert_eq!(cmd.module, Some("MyApp".to_string()));
    }
}
