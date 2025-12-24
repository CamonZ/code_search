use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

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
    db: &dyn Database,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern, Some(function_pattern)])?;

    // Build conditions using query builders
    let fn_cond = ConditionBuilder::new("name", "function_pattern").build(use_regex);
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

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

    let mut params = QueryParams::new()
        .with_str("function_pattern", function_pattern)
        .with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 11 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(file) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(2).unwrap(), 0);
            let start_line = extract_i64(row.get(3).unwrap(), 0);
            let end_line = extract_i64(row.get(4).unwrap(), 0);
            let Some(module) = extract_string(row.get(5).unwrap()) else {
                continue;
            };
            let kind = extract_string_or(row.get(6).unwrap(), "");
            let Some(name) = extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let arity = extract_i64(row.get(8).unwrap(), 0);
            let pattern = extract_string_or(row.get(9).unwrap(), "");
            let guard = extract_string_or(row.get(10).unwrap(), "");

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
