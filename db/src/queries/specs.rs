use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum SpecsError {
    #[error("Specs query failed: {message}")]
    QueryFailed { message: String },
}

/// A spec or callback definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecDef {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub full: String,
}

pub fn find_specs(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<SpecDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = OptionalConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(function_pattern.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, arity, kind, line, inputs_string, return_string, full] :=
            *specs{{project, module, name, arity, kind, line, inputs_string, return_string, full}},
            project == $project,
            {module_cond}
            {function_cond}
            {kind_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert(
        "module_pattern".to_string(),
        DataValue::Str(module_pattern.into()),
    );

    if let Some(func) = function_pattern {
        params.insert("function_pattern".to_string(), DataValue::Str(func.into()));
    }

    if let Some(kind) = kind_filter {
        params.insert("kind".to_string(), DataValue::Str(kind.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| SpecsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 9 {
            let Some(project) = extract_string(&row[0]) else {
                continue;
            };
            let Some(module) = extract_string(&row[1]) else {
                continue;
            };
            let Some(name) = extract_string(&row[2]) else {
                continue;
            };
            let arity = extract_i64(&row[3], 0);
            let Some(kind) = extract_string(&row[4]) else {
                continue;
            };
            let line = extract_i64(&row[5], 0);
            let inputs_string = extract_string(&row[6]).unwrap_or_default();
            let return_string = extract_string(&row[7]).unwrap_or_default();
            let full = extract_string(&row[8]).unwrap_or_default();

            results.push(SpecDef {
                project,
                module,
                name,
                arity,
                kind,
                line,
                inputs_string,
                return_string,
                full,
            });
        }
    }

    Ok(results)
}
