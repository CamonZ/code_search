//! Unified call graph queries for finding function calls.
//!
//! This module provides a single query function that can find calls in either direction:
//! - `From`: Find all calls made BY the matched functions (outgoing calls)
//! - `To`: Find all calls made TO the matched functions (incoming calls)

use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_call_from_row, run_query, CallRowLayout, Params};
use crate::types::Call;
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum CallsError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

/// Direction of call graph traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Find calls FROM the matched functions (what does this function call?)
    From,
    /// Find calls TO the matched functions (who calls this function?)
    To,
}

impl CallDirection {
    /// Returns the field names to filter on based on direction
    fn filter_fields(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            CallDirection::From => ("caller_module", "caller_name", "caller_arity"),
            CallDirection::To => ("callee_module", "callee_function", "callee_arity"),
        }
    }

    /// Returns the ORDER BY clause based on direction
    fn order_clause(&self) -> &'static str {
        match self {
            CallDirection::From => {
                "caller_module, caller_name, caller_arity, call_line, callee_module, callee_function, callee_arity"
            }
            CallDirection::To => {
                "callee_module, callee_function, callee_arity, caller_module, caller_name, caller_arity"
            }
        }
    }
}

/// Find calls in the specified direction.
///
/// - `From`: Returns all calls made by functions matching the pattern
/// - `To`: Returns all calls to functions matching the pattern
pub fn find_calls(
    db: &cozo::DbInstance,
    direction: CallDirection,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    let (module_field, function_field, arity_field) = direction.filter_fields();
    let order_clause = direction.order_clause();

    // Build conditions using the appropriate field names
    let module_cond =
        ConditionBuilder::new(module_field, "module_pattern").build(use_regex);
    let function_cond =
        OptionalConditionBuilder::new(function_field, "function_pattern")
            .with_leading_comma()
            .with_regex()
            .build_with_regex(function_pattern.is_some(), use_regex);
    let arity_cond = OptionalConditionBuilder::new(arity_field, "arity")
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
        :order {order_clause}
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert(
        "module_pattern".to_string(),
        DataValue::Str(module_pattern.into()),
    );
    if let Some(fn_pat) = function_pattern {
        params.insert(
            "function_pattern".to_string(),
            DataValue::Str(fn_pat.into()),
        );
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::from(a));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| CallsError::QueryFailed {
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
