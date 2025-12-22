use std::error::Error;

use serde::Serialize;

use super::ComplexityCmd;
use crate::commands::Execute;
use db::queries::complexity::find_complexity_metrics;
use db::types::ModuleCollectionResult;

/// A single complexity metric entry
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityEntry {
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub complexity: i64,
    pub max_nesting_depth: i64,
    pub lines: i64,
}

impl Execute for ComplexityCmd {
    type Output = ModuleCollectionResult<ComplexityEntry>;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let metrics = find_complexity_metrics(
            db,
            self.min,
            self.min_depth,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.exclude_generated,
            self.common.limit,
        )?;

        let total_items = metrics.len();

        // Group by module
        let items = crate::utils::group_by_module(metrics, |metric| {
            let entry = ComplexityEntry {
                name: metric.name,
                arity: metric.arity,
                line: metric.line,
                complexity: metric.complexity,
                max_nesting_depth: metric.max_nesting_depth,
                lines: metric.lines,
            };
            (metric.module, entry)
        });

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
    fn test_complexity_cmd_structure() {
        let cmd = ComplexityCmd {
            min: 10,
            min_depth: 3,
            exclude_generated: false,
            module: Some("MyApp".to_string()),
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 20,
            },
        };

        assert_eq!(cmd.min, 10);
        assert_eq!(cmd.min_depth, 3);
        assert!(!cmd.exclude_generated);
        assert_eq!(cmd.module, Some("MyApp".to_string()));
    }
}
