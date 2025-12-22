use std::error::Error;

use cozo::{DataValue, Num};
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Location query failed: {message}")]
    QueryFailed { message: String },
}

/// A function location result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionLocation {
    pub project: String,
    pub file: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub arity: i64,
    pub pattern: String,
    pub guard: String,
}

pub fn find_locations(
    db: &cozo::DbInstance,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    // Build the query based on whether we're using regex or exact match
    let fn_cond = if use_regex {
        "regex_matches(name, $function_pattern)".to_string()
    } else {
        "name == $function_pattern".to_string()
    };

    let module_cond = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", module == $module_pattern".to_string(),
        None => String::new(),
    };

    let arity_cond = if arity.is_some() {
        ", arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard] :=
            *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
            {fn_cond}
            {module_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity, line
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(mod_pat) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(mod_pat.into()));
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::Num(Num::Int(a)));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 11 {
            // Order matches query: project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(file) = extract_string(&row[1]) else { continue };
            let line = extract_i64(&row[2], 0);
            let start_line = extract_i64(&row[3], 0);
            let end_line = extract_i64(&row[4], 0);
            let Some(module) = extract_string(&row[5]) else { continue };
            let kind = extract_string_or(&row[6], "");
            let Some(name) = extract_string(&row[7]) else { continue };
            let arity = extract_i64(&row[8], 0);
            let pattern = extract_string_or(&row[9], "");
            let guard = extract_string_or(&row[10], "");

            results.push(FunctionLocation {
                project,
                file,
                line,
                start_line,
                end_line,
                module,
                kind,
                name,
                arity,
                pattern,
                guard,
            });
        }
    }

    Ok(results)
}
