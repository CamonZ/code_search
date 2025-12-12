use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_call_from_row, run_query, CallRowLayout, Params};
use crate::types::Call;

#[derive(Error, Debug)]
pub enum CallsFromError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

pub fn find_calls_from(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    // Build conditions for the caller using helpers
    let module_cond = crate::utils::ConditionBuilder::new("caller_module", "module_pattern").build(use_regex);
    let function_cond = crate::utils::OptionalConditionBuilder::new("caller_name", "function_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(function_pattern.is_some(), use_regex);
    let arity_cond = crate::utils::OptionalConditionBuilder::new("caller_arity", "arity")
        .with_leading_comma()
        .build(arity.is_some());

    let project_cond = ", project == $project";

    // Join calls with function_locations to get caller's arity and line range
    // The caller_function in calls includes arity suffix (e.g., "foo/2"), while function_locations
    // stores just the name. We use starts_with to match and verify with line range.
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

        :order caller_module, caller_name, caller_arity, call_line, callee_module, callee_function, callee_arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    if let Some(fn_pat) = function_pattern {
        params.insert("function_pattern".to_string(), DataValue::Str(fn_pat.into()));
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::from(a));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| CallsFromError::QueryFailed {
        message: e.to_string(),
    })?;

    let layout = CallRowLayout::from_headers(&rows.headers)?;
    let results = rows
        .rows
        .iter()
        .filter_map(|row| extract_call_from_row(row, &layout))
        .collect();

    Ok(results)
}
