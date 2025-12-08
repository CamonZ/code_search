use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

#[derive(Error, Debug)]
pub enum DependsOnError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// A module dependency with call count
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDependency {
    pub module: String,
    pub call_count: i64,
}

pub fn find_dependencies(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<ModuleDependency>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(caller_module, $module_pattern)"
    } else {
        "caller_module == $module_pattern"
    };

    let project_cond = ", project == $project";

    // Aggregate calls by callee module, excluding self-references
    // In CozoDB, count(caller_module) counts occurrences grouped by callee_module
    let script = format!(
        r#"
        ?[callee_module, count(caller_module)] :=
            *calls{{project, caller_module, callee_module}},
            {module_cond},
            caller_module != callee_module
            {project_cond}
        :order -count(caller_module), callee_module
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| DependsOnError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 2 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let call_count = extract_i64(&row[1], 0);

            results.push(ModuleDependency { module, call_count });
        }
    }

    Ok(results)
}
