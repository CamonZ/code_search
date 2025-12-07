use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::DependedByCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum DependedByError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// A module that depends on the target, with call count
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDependent {
    pub module: String,
    pub call_count: i64,
}

/// Result of the depended-by command execution
#[derive(Debug, Default, Serialize)]
pub struct DependedByResult {
    pub target_module: String,
    pub dependents: Vec<ModuleDependent>,
}

impl Execute for DependedByCmd {
    type Output = DependedByResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = DependedByResult {
            target_module: self.module.clone(),
            ..Default::default()
        };

        result.dependents = find_dependents(
            &db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_dependents(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<ModuleDependent>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(callee_module, $module_pattern)"
    } else {
        "callee_module == $module_pattern"
    };

    let project_cond = ", project == $project";

    // Aggregate calls by caller module, excluding self-references
    // In CozoDB, count(callee_module) counts occurrences grouped by caller_module
    let script = format!(
        r#"
        ?[caller_module, count(callee_module)] :=
            *calls{{project, caller_module, callee_module}},
            {module_cond},
            caller_module != callee_module
            {project_cond}
        :order -count(callee_module), caller_module
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| DependedByError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 2 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let call_count = extract_i64(&row[1], 0);

            results.push(ModuleDependent { module, call_count });
        }
    }

    Ok(results)
}
