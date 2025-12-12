use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

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
    // Build module filter
    let module_filter = if use_regex {
        "regex_matches(module, $module_pattern)"
    } else {
        "module == $module_pattern"
    };

    // Build function filter
    let function_filter = match function_pattern {
        Some(_) if use_regex => ", regex_matches(name, $function_pattern)",
        Some(_) => ", str_includes(name, $function_pattern)",
        None => "",
    };

    // Build kind filter
    let kind_filter_sql = match kind_filter {
        Some(_) => ", kind == $kind",
        None => "",
    };

    let script = format!(
        r#"
        ?[project, module, name, arity, kind, line, inputs_string, return_string, full] :=
            *specs{{project, module, name, arity, kind, line, inputs_string, return_string, full}},
            project == $project,
            {module_filter}
            {function_filter}
            {kind_filter_sql}

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
