use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

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
    db: &dyn Database,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), name_filter])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let name_cond = OptionalConditionBuilder::new("name", "name_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(name_filter.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, kind, params, line, definition] :=
            *types{{project, module, name, kind, params, line, definition}},
            project == $project,
            {module_cond}
            {name_cond}
            {kind_cond}

        :order module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_str("module_pattern", module_pattern);

    if let Some(name) = name_filter {
        params = params.with_str("name_pattern", name);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = run_query(db, &script, params).map_err(|e| TypesError::QueryFailed {
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
            let Some(kind) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let params_str = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(5).unwrap(), 0);
            let definition = extract_string(row.get(6).unwrap()).unwrap_or_default();

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
