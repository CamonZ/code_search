use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::types::{Call, FunctionRef};

#[derive(Error, Debug)]
pub enum TraceError {
    #[error("Trace query failed: {message}")]
    QueryFailed { message: String },
}

pub fn trace_calls(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    // Build the starting conditions for the recursive query using helpers
    let module_cond = crate::utils::ConditionBuilder::new("caller_module", "module_pattern").build(use_regex);
    let function_cond = crate::utils::ConditionBuilder::new("caller_name", "function_pattern").build(use_regex);
    let arity_cond = crate::utils::OptionalConditionBuilder::new("callee_arity", "arity")
        .when_none("true")
        .build(arity.is_some());

    // Recursive query to trace call chains, joined with function_locations for caller metadata
    // Base case: direct calls from the starting function
    // Recursive case: calls from functions we've already found
    // Filter out struct calls (callee_function != '%')
    let script = format!(
        r#"
        # Base case: calls from the starting function, joined with function_locations
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
            {module_cond},
            {function_cond},
            project == $project,
            {arity_cond},
            depth = 1

        # Recursive case: calls from callees we've found
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[prev_depth, _, _, _, _, _, _, prev_callee_module, prev_callee_function, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            caller_module == prev_callee_module,
            starts_with(caller_function, caller_name),
            starts_with(caller_function, prev_callee_function),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
            prev_depth < {max_depth},
            depth = prev_depth + 1,
            project == $project

        ?[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line]

        :order depth, caller_module, caller_name, caller_arity, call_line, callee_module, callee_function, callee_arity
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

    let rows = run_query(db, &script, params).map_err(|e| TraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 12 {
            let depth = extract_i64(&row[0], 0);
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
                call_type: None,
                depth: Some(depth),
            });
        }
    }

    Ok(results)
}
