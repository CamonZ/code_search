use std::collections::HashMap;
use std::error::Error;

use serde::Serialize;

use super::LargeFunctionsCmd;
use crate::commands::Execute;
use db::queries::large_functions::find_large_functions;
use db::types::{ModuleCollectionResult, ModuleGroup};

/// A single large function entry
#[derive(Debug, Clone, Serialize)]
pub struct LargeFunctionEntry {
    pub name: String,
    pub arity: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub file: String,
}

impl Execute for LargeFunctionsCmd {
    type Output = ModuleCollectionResult<LargeFunctionEntry>;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let large_functions = find_large_functions(
            db,
            self.min_lines,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.include_generated,
            self.common.limit,
        )?;

        let total_items = large_functions.len();

        // Group by module while preserving sort order (largest functions first)
        // Track module order separately to maintain insertion order
        let mut module_order: Vec<String> = Vec::new();
        let mut module_map: HashMap<String, (String, Vec<LargeFunctionEntry>)> = HashMap::new();

        for func in large_functions {
            let entry = LargeFunctionEntry {
                name: func.name,
                arity: func.arity,
                start_line: func.start_line,
                end_line: func.end_line,
                lines: func.lines,
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

        let items: Vec<ModuleGroup<LargeFunctionEntry>> = module_order
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
    fn test_large_functions_cmd_structure() {
        let cmd = LargeFunctionsCmd {
            min_lines: 100,
            include_generated: false,
            module: Some("MyApp".to_string()),
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 20,
            },
        };

        assert_eq!(cmd.min_lines, 100);
        assert!(!cmd.include_generated);
        assert_eq!(cmd.module, Some("MyApp".to_string()));
    }
}
