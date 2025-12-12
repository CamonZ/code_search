use std::error::Error;

use cozo::{DataValue, Num};
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::types::{Call, FunctionRef};

#[derive(Error, Debug)]
pub enum CallsToError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

pub fn find_calls_to(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    // Build conditions for the callee (target) using helpers
    let module_cond = crate::utils::ConditionBuilder::new("callee_module", "module_pattern").build(use_regex);
    let function_cond = crate::utils::OptionalConditionBuilder::new("callee_function", "function_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(function_pattern.is_some(), use_regex);
    let arity_cond = crate::utils::OptionalConditionBuilder::new("callee_arity", "arity")
        .with_leading_comma()
        .build(arity.is_some());

    let project_cond = ", project == $project";

    // Join calls with function_locations to get caller's arity and line range
    // Filter out struct calls (callee_function == '%')
    let script = format!(
        r#"
        ?[project, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line, call_type] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line, call_type, caller_kind}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
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
            let Some(_project) = extract_string(&row[0]) else { continue };
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_name) = extract_string(&row[2]) else { continue };
            let caller_arity = extract_i64(&row[3], 0);
            let caller_kind = extract_string_or(&row[4], "");
            let caller_start_line = extract_i64(&row[5], 0);
            let caller_end_line = extract_i64(&row[6], 0);
            let Some(callee_module) = extract_string(&row[7]) else { continue };
            let Some(callee_name) = extract_string(&row[8]) else { continue };
            let callee_arity = extract_i64(&row[9], 0);
            let Some(file) = extract_string(&row[10]) else { continue };
            let line = extract_i64(&row[11], 0);
            let call_type = extract_string_or(&row[12], "remote");

            let caller = FunctionRef::with_definition(
                caller_module,
                caller_name,
                caller_arity,
                caller_kind,
                &file,
                caller_start_line,
                caller_end_line,
            );

            // Callee doesn't have definition info from this query
            let callee = FunctionRef::new(callee_module, callee_name, callee_arity);

            results.push(Call {
                caller,
                callee,
                line,
                call_type: Some(call_type),
                depth: None,
            });
        }
    }

    Ok(results)
}
