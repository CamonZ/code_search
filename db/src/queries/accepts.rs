use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum AcceptsError {
    #[error("Accepts query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with its input type specification
#[derive(Debug, Clone, Serialize)]
pub struct AcceptsEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub line: i64,
}

pub fn find_accepts(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<AcceptsEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build conditions using query builders
    let pattern_cond = ConditionBuilder::new("inputs_string", "pattern").build(use_regex);
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        ?[project, module, name, arity, inputs_string, return_string, line] :=
            *specs{{project, module, name, arity, inputs_string, return_string, line}},
            project == $project,
            {pattern_cond}
            {module_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    let result = run_query(db, &script, params).map_err(|e| AcceptsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 7 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let arity = extract_i64(row.get(3).unwrap(), 0);
            let inputs_string = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let return_string = extract_string(row.get(5).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(6).unwrap(), 0);

            results.push(AcceptsEntry {
                project,
                module,
                name,
                arity,
                inputs_string,
                return_string,
                line,
            });
        }
    }

    Ok(results)
}
