use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

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
    db: &dyn DatabaseBackend,
    min_complexity: i64,
    min_depth: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<ComplexityMetric>, Box<dyn Error>> {
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

    let script = format!(
        r#"
        ?[module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, lines, generated_by] :=
            *function_locations{{project, module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, generated_by}},
            project == $project,
            complexity >= $min_complexity,
            max_nesting_depth >= $min_depth,
            lines = end_line - start_line + 1
            {module_filter}
            {generated_filter}

        :order -complexity, module, name
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert("min_complexity".to_string(), DataValue::from(min_complexity));
    params.insert("min_depth".to_string(), DataValue::from(min_depth));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| ComplexityError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 10 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let line = extract_i64(&row[3], 0);
            let complexity = extract_i64(&row[4], 0);
            let max_nesting_depth = extract_i64(&row[5], 0);
            let start_line = extract_i64(&row[6], 0);
            let end_line = extract_i64(&row[7], 0);
            let lines = extract_i64(&row[8], 0);
            let Some(generated_by) = extract_string(&row[9]) else { continue };

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
