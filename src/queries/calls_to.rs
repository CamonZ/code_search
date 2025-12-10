use std::error::Error;

use cozo::{DataValue, Num};
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};

#[derive(Error, Debug)]
pub enum CallsToError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

/// A single call edge (incoming to the callee)
#[derive(Debug, Clone, Serialize)]
pub struct CallEdge {
    pub project: String,
    pub caller_module: String,
    pub caller_function: String,
    pub caller_kind: String,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
    pub call_type: String,
}

pub fn find_calls_to(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<CallEdge>, Box<dyn Error>> {
    // Build conditions for the callee (target)
    let module_cond = if use_regex {
        "regex_matches(callee_module, $module_pattern)".to_string()
    } else {
        "callee_module == $module_pattern".to_string()
    };

    let function_cond = match function_pattern {
        Some(_) if use_regex => ", regex_matches(callee_function, $function_pattern)".to_string(),
        Some(_) => ", callee_function == $function_pattern".to_string(),
        None => String::new(),
    };

    let arity_cond = if arity.is_some() {
        ", callee_arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, caller_module, caller_function, caller_kind, callee_module, callee_function, callee_arity, file, line, call_type] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, call_type, caller_kind}},
            {module_cond}
            {function_cond}
            {arity_cond}
            {project_cond}
        :order caller_module, caller_function
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    if let Some(fn_pat) = function_pattern {
        params.insert("function_pattern".to_string(), DataValue::Str(fn_pat.into()));
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::Num(Num::Int(a)));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| CallsToError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 10 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let caller_kind = extract_string_or(&row[3], "");
            let Some(callee_module) = extract_string(&row[4]) else { continue };
            let Some(callee_function) = extract_string(&row[5]) else { continue };
            let callee_arity = extract_i64(&row[6], 0);
            let Some(file) = extract_string(&row[7]) else { continue };
            let line = extract_i64(&row[8], 0);
            let call_type = extract_string_or(&row[9], "remote");

            results.push(CallEdge {
                project,
                caller_module,
                caller_function,
                caller_kind,
                callee_module,
                callee_function,
                callee_arity,
                file,
                line,
                call_type,
            });
        }
    }

    Ok(results)
}
