use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum ReverseTraceError {
    #[error("Reverse trace query failed: {message}")]
    QueryFailed { message: String },
}

/// A single step in the reverse call chain
#[derive(Debug, Clone, Serialize)]
pub struct ReverseTraceStep {
    pub depth: i64,
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
}

pub fn reverse_trace_calls(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<ReverseTraceStep>, Box<dyn Error>> {
    // Build the starting conditions for the recursive query using helpers
    // For reverse trace, we match on the callee (target)
    let module_cond = ConditionBuilder::new("callee_module", "module_pattern").build(use_regex);
    let function_cond = ConditionBuilder::new("callee_function", "function_pattern").build(use_regex);
    let arity_cond = OptionalConditionBuilder::new("callee_arity", "arity")
        .when_none("true")
        .build(arity.is_some());

    // Recursive query to trace call chains backwards, joined with function_locations for caller metadata
    // Base case: calls TO the target function
    // Recursive case: calls TO the callers we've found
    let script = format!(
        r#"
        # Base case: calls to the target function, joined with function_locations
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            {module_cond},
            {function_cond},
            project == $project,
            {arity_cond},
            depth = 1

        # Recursive case: calls to the callers we've found
        # Note: prev_caller_function has arity suffix (e.g., "foo/2") but callee_function doesn't (e.g., "foo")
        # So we use starts_with to match prev_caller_function starting with callee_function
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[prev_depth, prev_caller_module, prev_caller_name, prev_caller_arity, _, _, _, _, _, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            callee_module == prev_caller_module,
            callee_function == prev_caller_name,
            callee_arity == prev_caller_arity,
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
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

    let result = run_query(db, &script, params).map_err(|e| ReverseTraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 12 {
            let depth = extract_i64(row.get(0).unwrap(), 0);
            let Some(caller_module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(caller_function) = extract_string(row.get(2).unwrap()) else { continue };
            let caller_arity = extract_i64(row.get(3).unwrap(), 0);
            let caller_kind = extract_string_or(row.get(4).unwrap(), "");
            let caller_start_line = extract_i64(row.get(5).unwrap(), 0);
            let caller_end_line = extract_i64(row.get(6).unwrap(), 0);
            let Some(callee_module) = extract_string(row.get(7).unwrap()) else { continue };
            let Some(callee_function) = extract_string(row.get(8).unwrap()) else { continue };
            let callee_arity = extract_i64(row.get(9).unwrap(), 0);
            let Some(file) = extract_string(row.get(10).unwrap()) else { continue };
            let line = extract_i64(row.get(11).unwrap(), 0);

            results.push(ReverseTraceStep {
                depth,
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
            });
        }
    }

    Ok(results)
}
