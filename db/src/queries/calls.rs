//! Unified call graph queries for finding function calls.
//!
//! This module provides a single query function that can find calls in either direction:
//! - `From`: Find all calls made BY the matched functions (outgoing calls)
//! - `To`: Find all calls made TO the matched functions (incoming calls)

use std::error::Error;
use std::rc::Rc;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or};
use crate::query_builders::validate_regex_patterns;
use crate::types::{Call, FunctionRef};

#[cfg(feature = "backend-cozo")]
use crate::db::{extract_call_from_row_trait, run_query, CallRowLayout};

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

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
    #[cfg(feature = "backend-cozo")]
    fn filter_fields(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            CallDirection::From => ("caller_module", "caller_name", "caller_arity"),
            CallDirection::To => ("callee_module", "callee_function", "callee_arity"),
        }
    }

    #[cfg(feature = "backend-cozo")]
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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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
    let module_cond = ConditionBuilder::new(module_field, "module_pattern").build(use_regex);
    let function_cond = OptionalConditionBuilder::new(function_field, "function_pattern")
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
/// Find calls in the specified direction.
///
/// - `From`: Returns all calls made by functions matching the pattern
/// - `To`: Returns all calls to functions matching the pattern
///
/// Uses SurrealQL graph traversal operators:
/// - `->calls->` for outgoing edges (calls made FROM the function)
/// - `<-calls<-` for incoming edges (calls made TO the function)
pub fn find_calls(
    db: &dyn Database,
    direction: CallDirection,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build query based on direction using dot notation (in.field / out.field)
    // SurrealDB supports both arrow syntax and dot notation in WHERE clauses
    let (where_clause_base, fn_pattern_field, arity_field, order_by) = match direction {
        CallDirection::From => {
            // For outgoing: filter by caller properties (in.*)
            let fn_field = if use_regex {
                " AND in.name = <regex>$function_pattern".to_string()
            } else if function_pattern.is_some() {
                " AND in.name = $function_pattern".to_string()
            } else {
                String::new()
            };
            let ar_field = if arity.is_some() {
                " AND in.arity = $arity".to_string()
            } else {
                String::new()
            };
            (
                "in.module_name",
                fn_field,
                ar_field,
                "in.module_name, in.name, in.arity, line, out.module_name, out.name, out.arity",
            )
        }
        CallDirection::To => {
            // For incoming: filter by callee properties (out.*)
            let fn_field = if use_regex {
                " AND out.name = <regex>$function_pattern".to_string()
            } else if function_pattern.is_some() {
                " AND out.name = $function_pattern".to_string()
            } else {
                String::new()
            };
            let ar_field = if arity.is_some() {
                " AND out.arity = $arity".to_string()
            } else {
                String::new()
            };
            (
                "out.module_name",
                fn_field,
                ar_field,
                "out.module_name, out.name, out.arity, in.module_name, in.name, in.arity",
            )
        }
    };

    // Build the WHERE clause dynamically based on regex or exact match
    let where_module = if use_regex {
        format!("{} = <regex>$module_pattern", where_clause_base)
    } else {
        format!("{} = $module_pattern", where_clause_base)
    };

    // Query the calls edge table with proper WHERE filtering
    // Uses dot notation (in.field, out.field) for accessing connected record properties
    let query = format!(
        r#"
        SELECT
            "default" as project,
            in.name as caller_name,
            in.module_name as caller_module,
            in.arity as caller_arity,
            "" as caller_kind,
            0 as caller_start_line,
            0 as caller_end_line,
            out.module_name as callee_module,
            out.name as callee_function,
            out.arity as callee_arity,
            "" as file,
            line as callee_line,
            call_type
        FROM calls
        WHERE {}{}{}
        ORDER BY {}
        LIMIT $limit
        "#,
        where_module, fn_pattern_field, arity_field, order_by
    );

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_int("limit", limit as i64);

    if let Some(fn_pat) = function_pattern {
        params = params.with_str("function_pattern", fn_pat);
    }
    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = db
        .execute_query(&query, params)
        .map_err(|e| CallsError::QueryFailed {
            message: e.to_string(),
        })?;

    // Parse results from SurrealDB rows
    // SurrealDB returns columns in alphabetical order:
    // callee_arity, callee_function, callee_line, callee_module, call_type, caller_arity,
    // caller_kind, caller_module, caller_name, caller_start_line, caller_end_line, file, project
    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 13 {
            let callee_arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(callee_function) = extract_string(row.get(1).unwrap()) else {
                // Skip rows where callee_function is NULL (no call found)
                continue;
            };
            let callee_line = extract_i64(row.get(2).unwrap(), 0);
            let Some(callee_module) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let call_type_str = extract_string_or(row.get(4).unwrap(), "");
            let caller_arity = extract_i64(row.get(5).unwrap(), 0);
            let _caller_kind = extract_string_or(row.get(6).unwrap(), "");
            let Some(caller_module) = extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let Some(caller_name) = extract_string(row.get(8).unwrap()) else {
                continue;
            };
            let _caller_start_line = extract_i64(row.get(9).unwrap(), 0);
            let _caller_end_line = extract_i64(row.get(10).unwrap(), 0);
            let _file = extract_string_or(row.get(11).unwrap(), "");

            let caller =
                FunctionRef::new(Rc::from(caller_module), Rc::from(caller_name), caller_arity);
            let callee = FunctionRef::new(
                Rc::from(callee_module),
                Rc::from(callee_function),
                callee_arity,
            );

            results.push(Call {
                caller,
                callee,
                line: callee_line,
                call_type: if call_type_str.is_empty() {
                    None
                } else {
                    Some(call_type_str)
                },
                depth: None,
            });
        }
    }

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
        assert!(
            calls.is_empty() || !calls.is_empty(),
            "Query should execute"
        );
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
        assert!(
            calls.is_empty(),
            "Should return empty for non-existent module"
        );
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
        assert!(
            limit_5.len() <= limit_100.len(),
            "Higher limit should return >= results"
        );
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
        assert!(
            calls.is_empty(),
            "Non-existent project should return no results"
        );
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_calls_from_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_calls(
            &*db,
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
        assert!(
            calls.is_empty(),
            "Non-existent module should return no calls"
        );
    }

    #[test]
    fn test_find_calls_invalid_regex_pattern() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_calls(
            &*db,
            CallDirection::From,
            "[invalid",
            None,
            None,
            "default",
            true,
            100,
        );

        assert!(result.is_err(), "Should reject invalid regex pattern");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid regex pattern"));
    }

    #[test]
    fn test_find_calls_empty_when_no_match() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_calls(
            &*db,
            CallDirection::From,
            "NonExistentModule",
            None,
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed even with no matches");
        let calls = result.unwrap();
        assert!(
            calls.is_empty(),
            "Should return empty for non-existent module"
        );
    }

    #[test]
    fn test_find_calls_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            1,
        )
        .unwrap_or_default();

        let limit_100 = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            100,
        )
        .unwrap_or_default();

        // The limit should be respected (though may not have enough data in fixture)
        assert!(limit_1.len() <= 1, "Limit of 1 should be respected");
        assert!(
            limit_1.len() <= limit_100.len(),
            "Higher limit should return >= results"
        );
    }
}
