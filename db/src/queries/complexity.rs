use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum ComplexityError {
    #[error("Complexity query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with complexity metrics
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityMetric {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub complexity: i64,
    pub max_nesting_depth: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub generated_by: String,
}

pub fn find_complexity_metrics(
    db: &dyn Database,
    min_complexity: i64,
    min_depth: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<ComplexityMetric>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

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

    let script = format!(
        r#"
        ?[module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, lines, generated_by] :=
            *function_locations{{project, module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, generated_by}},
            project == $project,
            complexity >= $min_complexity,
            max_nesting_depth >= $min_depth,
            lines = end_line - start_line + 1
            {module_cond}
            {generated_filter}

        :order -complexity, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_int("min_complexity", min_complexity)
        .with_int("min_depth", min_depth);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| ComplexityError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 10 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let line = extract_i64(row.get(3).unwrap(), 0);
            let complexity = extract_i64(row.get(4).unwrap(), 0);
            let max_nesting_depth = extract_i64(row.get(5).unwrap(), 0);
            let start_line = extract_i64(row.get(6).unwrap(), 0);
            let end_line = extract_i64(row.get(7).unwrap(), 0);
            let lines = extract_i64(row.get(8).unwrap(), 0);
            let Some(generated_by) = extract_string(row.get(9).unwrap()) else { continue };

            results.push(ComplexityMetric {
                module,
                name,
                arity,
                line,
                complexity,
                max_nesting_depth,
                start_line,
                end_line,
                lines,
                generated_by,
            });
        }
    }

    Ok(results)
}
