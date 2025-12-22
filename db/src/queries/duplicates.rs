use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

#[derive(Error, Debug)]
pub enum DuplicatesError {
    #[error("Duplicates query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that has a duplicate implementation (same AST or source hash)
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateFunction {
    pub hash: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub file: String,
}

pub fn find_duplicates(
    db: &cozo::DbInstance,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
    use_exact: bool,
    exclude_generated: bool,
) -> Result<Vec<DuplicateFunction>, Box<dyn Error>> {
    // Choose hash field based on exact flag
    let hash_field = if use_exact { "source_sha" } else { "ast_sha" };

    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

    // Build optional generated filter
    let generated_filter = if exclude_generated {
        ", generated_by == \"\"".to_string()
    } else {
        String::new()
    };

    // Query to find duplicate hashes and their functions
    let script = format!(
        r#"
        # Find hashes that appear more than once (count unique functions per hash)
        hash_counts[{hash_field}, count(module)] :=
            *function_locations{{project, module, name, arity, {hash_field}, generated_by}},
            project == $project,
            {hash_field} != ""
            {generated_filter}

        # Get all functions with duplicate hashes
        ?[{hash_field}, module, name, arity, line, file] :=
            *function_locations{{project, module, name, arity, line, file, {hash_field}, generated_by}},
            hash_counts[{hash_field}, cnt],
            cnt > 1,
            project == $project
            {module_filter}
            {generated_filter}

        :order {hash_field}, module, name, arity
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| DuplicatesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(hash) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let line = extract_i64(&row[4], 0);
            let Some(file) = extract_string(&row[5]) else { continue };

            results.push(DuplicateFunction {
                hash,
                module,
                name,
                arity,
                line,
                file,
            });
        }
    }

    Ok(results)
}
