use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum LargeFunctionsError {
    #[error("Large functions query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with line count information
#[derive(Debug, Clone, Serialize)]
pub struct LargeFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub file: String,
    pub generated_by: String,
}

pub fn find_large_functions(
    db: &dyn Database,
    min_lines: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<LargeFunction>, Box<dyn Error>> {
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
        ?[module, name, arity, start_line, end_line, lines, file, generated_by] :=
            *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
            project == $project,
            lines = end_line - start_line + 1,
            lines >= $min_lines
            {module_cond}
            {generated_filter}

        :order -lines, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_int("min_lines", min_lines);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| LargeFunctionsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 8 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let start_line = extract_i64(row.get(3).unwrap(), 0);
            let end_line = extract_i64(row.get(4).unwrap(), 0);
            let lines = extract_i64(row.get(5).unwrap(), 0);
            let Some(file) = extract_string(row.get(6).unwrap()) else { continue };
            let Some(generated_by) = extract_string(row.get(7).unwrap()) else { continue };

            results.push(LargeFunction {
                module,
                name,
                arity,
                start_line,
                end_line,
                lines,
                file,
                generated_by,
            });
        }
    }

    Ok(results)
}
