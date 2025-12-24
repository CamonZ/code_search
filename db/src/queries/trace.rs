use std::error::Error;
use std::rc::Rc;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::types::{Call, FunctionRef};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum TraceError {
    #[error("Trace query failed: {message}")]
    QueryFailed { message: String },
}

pub fn trace_calls(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), Some(function_pattern)])?;

    // Build the starting conditions for the recursive query using helpers
    let module_cond = ConditionBuilder::new("caller_module", "module_pattern").build(use_regex);
    let function_cond = ConditionBuilder::new("caller_name", "function_pattern").build(use_regex);
    let arity_cond = OptionalConditionBuilder::new("caller_arity", "arity")
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

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("function_pattern", function_pattern)
        .with_str("project", project);

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| TraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 12 {
            let depth = extract_i64(row.get(0).unwrap(), 0);
            let Some(caller_module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(caller_name) = extract_string(row.get(2).unwrap()) else { continue };
            let caller_arity = extract_i64(row.get(3).unwrap(), 0);
            let caller_kind = extract_string_or(row.get(4).unwrap(), "");
            let caller_start_line = extract_i64(row.get(5).unwrap(), 0);
            let caller_end_line = extract_i64(row.get(6).unwrap(), 0);
            let Some(callee_module) = extract_string(row.get(7).unwrap()) else { continue };
            let Some(callee_name) = extract_string(row.get(8).unwrap()) else { continue };
            let callee_arity = extract_i64(row.get(9).unwrap(), 0);
            let Some(file) = extract_string(row.get(10).unwrap()) else { continue };
            let line = extract_i64(row.get(11).unwrap(), 0);

            let caller = FunctionRef::with_definition(
                Rc::from(caller_module.into_boxed_str()),
                Rc::from(caller_name.into_boxed_str()),
                caller_arity,
                Rc::from(caller_kind.into_boxed_str()),
                Rc::from(file.into_boxed_str()),
                caller_start_line,
                caller_end_line,
            );

            // Callee doesn't have definition info from this query
            let callee = FunctionRef::new(
                Rc::from(callee_module.into_boxed_str()),
                Rc::from(callee_name.into_boxed_str()),
                callee_arity,
            );

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
