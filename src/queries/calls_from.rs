use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};

#[derive(Error, Debug)]
pub enum CallsFromError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

/// A single call edge (outgoing from the caller)
#[derive(Debug, Clone, Serialize)]
pub struct CallEdge {
    pub project: String,
    pub caller_module: String,
    pub caller_function: String,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
    pub call_type: String,
}

pub fn find_calls_from(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<CallEdge>, Box<dyn Error>> {
    // Build conditions for the caller
    let module_cond = if use_regex {
        "regex_matches(caller_module, $module_pattern)".to_string()
    } else {
        "caller_module == $module_pattern".to_string()
    };

    let function_cond = match function_pattern {
        Some(_) if use_regex => ", regex_matches(caller_function, $function_pattern)".to_string(),
        Some(_) => ", caller_function == $function_pattern".to_string(),
        None => String::new(),
    };

    // Note: arity filtering for calls-from would need to join with function_locations
    // For now, we skip arity filtering on the caller side as calls table doesn't have caller_arity
    let _ = arity; // Acknowledge unused for now

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, call_type] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, call_type}},
            {module_cond}
            {function_cond}
            {project_cond}
        :order callee_module, callee_function, callee_arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    if let Some(fn_pat) = function_pattern {
        params.insert("function_pattern".to_string(), DataValue::Str(fn_pat.into()));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| CallsFromError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 9 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let Some(callee_module) = extract_string(&row[3]) else { continue };
            let Some(callee_function) = extract_string(&row[4]) else { continue };
            let callee_arity = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let line = extract_i64(&row[7], 0);
            let call_type = extract_string_or(&row[8], "remote");

            results.push(CallEdge {
                project,
                caller_module,
                caller_function,
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
