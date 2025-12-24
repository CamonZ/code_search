use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum ManyClausesError {
    #[error("Many clauses query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with clause count information
#[derive(Debug, Clone, Serialize)]
pub struct ManyClauses {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub clauses: i64,
    pub first_line: i64,
    pub last_line: i64,
    pub file: String,
    pub generated_by: String,
}

pub fn find_many_clauses(
    db: &dyn Database,
    min_clauses: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<ManyClauses>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    // Build optional generated filter
    let generated_filter = if include_generated {
        String::new()
    } else {
        ", generated_by == \"\"".to_string()
    };

    let script = format!(
        r#"
        clause_counts[module, name, arity, count(line), min(start_line), max(end_line), file, generated_by] :=
            *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
            project == $project
            {module_cond}
            {generated_filter}

        ?[module, name, arity, clauses, first_line, last_line, file, generated_by] :=
            clause_counts[module, name, arity, clauses, first_line, last_line, file, generated_by],
            clauses >= $min_clauses

        :order -clauses, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("project", project);
    params = params.with_int("min_clauses", min_clauses);
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| ManyClausesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 8 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let clauses = extract_i64(row.get(3).unwrap(), 0);
            let first_line = extract_i64(row.get(4).unwrap(), 0);
            let last_line = extract_i64(row.get(5).unwrap(), 0);
            let Some(file) = extract_string(row.get(6).unwrap()) else { continue };
            let Some(generated_by) = extract_string(row.get(7).unwrap()) else { continue };

            results.push(ManyClauses {
                module,
                name,
                arity,
                clauses,
                first_line,
                last_line,
                file,
                generated_by,
            });
        }
    }

    Ok(results)
}
