use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Function query failed: {message}")]
    QueryFailed { message: String },
}

/// A function signature
#[derive(Debug, Clone, Serialize)]
pub struct FunctionSignature {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub args: String,
    pub return_type: String,
}

pub fn find_functions(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    // Build query conditions using helpers
    let module_cond = crate::utils::ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = crate::utils::ConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .build(use_regex);
    let arity_cond = crate::utils::OptionalConditionBuilder::new("arity", "arity")
        .with_leading_comma()
        .build(arity.is_some());
    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, module, name, arity, args, return_type] :=
            *functions{{project, module, name, arity, args, return_type}},
            {module_cond}
            {function_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::from(a));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| FunctionError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let args = extract_string_or(&row[4], "");
            let return_type = extract_string_or(&row[5], "");

            results.push(FunctionSignature {
                project,
                module,
                name,
                arity,
                args,
                return_type,
            });
        }
    }

    Ok(results)
}
