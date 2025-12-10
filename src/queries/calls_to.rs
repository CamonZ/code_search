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
    pub caller_arity: i64,
    pub caller_kind: String,
    pub caller_start_line: i64,
    pub caller_end_line: i64,
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

    // Join calls with function_locations to get caller's arity and line range
    // The caller_function in calls includes arity suffix (e.g., "foo/2"), while function_locations
    // stores just the name. We use starts_with to match and verify with line range.
    let script = format!(
        r#"
        ?[project, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line, call_type] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line, call_type, caller_kind}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            {module_cond}
            {function_cond}
            {arity_cond}
            {project_cond}
        :order callee_module, callee_function, callee_arity, caller_module, caller_name, caller_arity
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
        if row.len() >= 13 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let caller_arity = extract_i64(&row[3], 0);
            let caller_kind = extract_string_or(&row[4], "");
            let caller_start_line = extract_i64(&row[5], 0);
            let caller_end_line = extract_i64(&row[6], 0);
            let Some(callee_module) = extract_string(&row[7]) else { continue };
            let Some(callee_function) = extract_string(&row[8]) else { continue };
            let callee_arity = extract_i64(&row[9], 0);
            let Some(file) = extract_string(&row[10]) else { continue };
            let line = extract_i64(&row[11], 0);
            let call_type = extract_string_or(&row[12], "remote");

            results.push(CallEdge {
                project,
                caller_module,
                caller_function,
                caller_arity,
                caller_kind,
                caller_start_line,
                caller_end_line,
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
