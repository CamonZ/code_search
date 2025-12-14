use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

#[derive(Error, Debug)]
pub enum StructUsageError {
    #[error("Struct usage query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that accepts or returns a specific type
#[derive(Debug, Clone, Serialize)]
pub struct StructUsageEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub line: i64,
}

pub fn find_struct_usage(
    db: &dyn DatabaseBackend,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<StructUsageEntry>, Box<dyn Error>> {
    // Build pattern matching function for both inputs and return
    let match_fn = if use_regex {
        "regex_matches(inputs_string, $pattern) or regex_matches(return_string, $pattern)"
    } else {
        "str_includes(inputs_string, $pattern) or str_includes(return_string, $pattern)"
    };

    // Build module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => "regex_matches(module, $module_pattern)",
        Some(_) => "str_includes(module, $module_pattern)",
        None => "true",
    };

    let script = format!(
        r#"
        ?[project, module, name, arity, inputs_string, return_string, line] :=
            *specs{{project, module, name, arity, inputs_string, return_string, line}},
            project == $project,
            {match_fn},
            {module_filter}

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

    let rows = run_query(db, &script, params).map_err(|e| StructUsageError::QueryFailed {
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

            results.push(StructUsageEntry {
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
