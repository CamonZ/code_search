use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
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
    db: &cozo::DbInstance,
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

    let mut params = Params::new();
    params.insert("pattern".to_string(), DataValue::Str(pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    if let Some(mod_pat) = module_pattern {
        params.insert(
            "module_pattern".to_string(),
            DataValue::Str(mod_pat.into()),
        );
    }

    let rows = run_query(db, &script, params).map_err(|e| AcceptsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 7 {
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
            let inputs_string = extract_string(&row[4]).unwrap_or_default();
            let return_string = extract_string(&row[5]).unwrap_or_default();
            let line = extract_i64(&row[6], 0);

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
