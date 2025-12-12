use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

#[derive(Error, Debug)]
pub enum TypesError {
    #[error("Types query failed: {message}")]
    QueryFailed { message: String },
}

/// A type definition (@type, @typep, @opaque)
#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub project: String,
    pub module: String,
    pub name: String,
    pub kind: String,
    pub params: String,
    pub line: i64,
    pub definition: String,
}

pub fn find_types(
    db: &cozo::DbInstance,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    // Build module filter
    let module_filter = if use_regex {
        "regex_matches(module, $module_pattern)"
    } else {
        "module == $module_pattern"
    };

    // Build name filter
    let name_filter_sql = match name_filter {
        Some(_) if use_regex => ", regex_matches(name, $name_pattern)",
        Some(_) => ", str_includes(name, $name_pattern)",
        None => "",
    };

    // Build kind filter
    let kind_filter_sql = match kind_filter {
        Some(_) => ", kind == $kind",
        None => "",
    };

    let script = format!(
        r#"
        ?[project, module, name, kind, params, line, definition] :=
            *types{{project, module, name, kind, params, line, definition}},
            project == $project,
            {module_filter}
            {name_filter_sql}
            {kind_filter_sql}

        :order module, name
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert(
        "module_pattern".to_string(),
        DataValue::Str(module_pattern.into()),
    );

    if let Some(name) = name_filter {
        params.insert("name_pattern".to_string(), DataValue::Str(name.into()));
    }

    if let Some(kind) = kind_filter {
        params.insert("kind".to_string(), DataValue::Str(kind.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| TypesError::QueryFailed {
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
            let Some(kind) = extract_string(&row[3]) else {
                continue;
            };
            let params_str = extract_string(&row[4]).unwrap_or_default();
            let line = extract_i64(&row[5], 0);
            let definition = extract_string(&row[6]).unwrap_or_default();

            results.push(TypeInfo {
                project,
                module,
                name,
                kind,
                params: params_str,
                line,
                definition,
            });
        }
    }

    Ok(results)
}
