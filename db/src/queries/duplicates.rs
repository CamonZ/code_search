use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

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
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
    use_exact: bool,
    exclude_generated: bool,
) -> Result<Vec<DuplicateFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Choose hash field based on exact flag
    let hash_field = if use_exact { "source_sha" } else { "ast_sha" };

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

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
            {module_cond}
            {generated_filter}

        :order {hash_field}, module, name, arity
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| DuplicatesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(hash) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(2).unwrap()) else { continue };
            let arity = extract_i64(row.get(3).unwrap(), 0);
            let line = extract_i64(row.get(4).unwrap(), 0);
            let Some(file) = extract_string(row.get(5).unwrap()) else { continue };

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
