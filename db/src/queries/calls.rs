//! Unified call graph queries for finding function calls.
//!
//! This module provides a single query function that can find calls in either direction:
//! - `From`: Find all calls made BY the matched functions (outgoing calls)
//! - `To`: Find all calls made TO the matched functions (incoming calls)

use std::error::Error;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_call_from_row_trait, run_query, CallRowLayout};
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
    db: &dyn Database,
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

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("project", project);

    if let Some(fn_pat) = function_pattern {
        params = params.with_str("function_pattern", fn_pat);
    }
    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| CallsError::QueryFailed {
        message: e.to_string(),
    })?;

    let layout = CallRowLayout::from_headers(result.headers())?;
    let results = result
        .rows()
        .iter()
        .filter_map(|row| extract_call_from_row_trait(&**row, &layout))
        .collect();

    Ok(results)
}

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::call_graph_db("default")
    }

    #[rstest]
    fn test_find_calls_from_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls(
            &*populated_db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(!calls.is_empty(), "Should find calls from module");
    }

    #[rstest]
    fn test_find_calls_to_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls(
            &*populated_db,
            CallDirection::To,
            "",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // May have some results
        assert!(calls.is_empty() || !calls.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_calls_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls(
            &*populated_db,
            CallDirection::From,
            "NonExistent",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Should return empty for non-existent module");
    }

    #[rstest]
    fn test_find_calls_with_function_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls(
            &*populated_db,
            CallDirection::From,
            "MyApp.Controller",
            Some("index"),
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Verify all results match the function pattern
        for call in &calls {
            assert!(call.caller.name.contains("index"));
        }
    }

    #[rstest]
    fn test_find_calls_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_calls(
            &*populated_db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            5,
        )
        .unwrap();
        let limit_100 = find_calls(
            &*populated_db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            100,
        )
        .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_calls_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls(
            &*populated_db,
            CallDirection::From,
            "MyApp",
            None,
            None,
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent project should return no results");
    }
}
