use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

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
    db: &dyn DatabaseBackend,
    min_lines: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<LargeFunction>, Box<dyn Error>> {
    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

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
            {module_filter}
            {generated_filter}

        :order -lines, module, name
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert("min_lines".to_string(), DataValue::from(min_lines));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| LargeFunctionsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let start_line = extract_i64(&row[3], 0);
            let end_line = extract_i64(&row[4], 0);
            let lines = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let Some(generated_by) = extract_string(&row[7]) else { continue };

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
