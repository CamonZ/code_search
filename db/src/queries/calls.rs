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
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build query based on direction using dot notation (in.field / out.field)
    // SurrealDB supports both arrow syntax and dot notation in WHERE clauses
    //
    // Note: SurrealDB has a quirk where combining `in.module_name = X AND in.name = Y`
    // in a WHERE clause returns 0 rows, but using `type::string(in.name) = Y` works.
    // This appears to be a SurrealDB edge-property access issue when multiple conditions
    // reference the same edge endpoint.
    let (where_clause_base, fn_pattern_field, arity_field, order_by) = match direction {
        CallDirection::From => {
            // For outgoing: filter by caller properties (in.*)
            // Only add function pattern condition if pattern is provided
            // Using type::string() to work around SurrealDB multi-condition quirk
            let fn_field = if use_regex && function_pattern.is_some() {
                " AND string::matches(in.name, $function_pattern)".to_string()
            } else if function_pattern.is_some() {
                " AND type::string(in.name) = $function_pattern".to_string()
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
            // Only add function pattern condition if pattern is provided
            // Using type::string() to work around SurrealDB multi-condition quirk
            let fn_field = if use_regex && function_pattern.is_some() {
                " AND string::matches(out.name, $function_pattern)".to_string()
            } else if function_pattern.is_some() {
                " AND type::string(out.name) = $function_pattern".to_string()
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
        format!("string::matches({}, $module_pattern)", where_clause_base)
    } else {
        format!("{} = $module_pattern", where_clause_base)
    };

    // Query the calls edge table with proper WHERE filtering
    // Uses dot notation (in.field, out.field) for accessing connected record properties
    // Uses caller_clause_id to get start_line/end_line from the specific clause
    let query = format!(
        r#"
        SELECT
            "default" as project,
            in.name as caller_name,
            in.module_name as caller_module,
            in.arity as caller_arity,
            in.kind as caller_kind,
            caller_clause_id.start_line as caller_start_line,
            caller_clause_id.end_line as caller_end_line,
            out.module_name as callee_module,
            out.name as callee_function,
            out.arity as callee_arity,
            in.file as file,
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
    // SurrealDB returns columns in alphabetical order by alias name:
    // 0: call_type, 1: callee_arity, 2: callee_function, 3: callee_line, 4: callee_module,
    // 5: caller_arity, 6: caller_end_line, 7: caller_kind, 8: caller_module, 9: caller_name,
    // 10: caller_start_line, 11: file, 12: project
    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 13 {
            let call_type_str = extract_string_or(row.get(0).unwrap(), "");
            let callee_arity = extract_i64(row.get(1).unwrap(), 0);
            let Some(callee_function) = extract_string(row.get(2).unwrap()) else {
                // Skip rows where callee_function is NULL (no call found)
                continue;
            };
            let callee_line = extract_i64(row.get(3).unwrap(), 0);
            let Some(callee_module) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let caller_arity = extract_i64(row.get(5).unwrap(), 0);
            let caller_end_line = extract_i64(row.get(6).unwrap(), 0);
            let caller_kind = extract_string_or(row.get(7).unwrap(), "");
            let Some(caller_module) = extract_string(row.get(8).unwrap()) else {
                continue;
            };
            let Some(caller_name) = extract_string(row.get(9).unwrap()) else {
                continue;
            };
            let caller_start_line = extract_i64(row.get(10).unwrap(), 0);
            let file = extract_string_or(row.get(11).unwrap(), "");

            // Build caller with definition info from caller_clause_id traversal
            let caller = if caller_start_line > 0 && caller_end_line > 0 && !caller_kind.is_empty() {
                FunctionRef::with_definition(
                    Rc::from(caller_module),
                    Rc::from(caller_name),
                    caller_arity,
                    Rc::from(caller_kind),
                    Rc::from(file),
                    caller_start_line,
                    caller_end_line,
                )
            } else {
                FunctionRef::new(Rc::from(caller_module), Rc::from(caller_name), caller_arity)
            };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_calls_from_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls(
            &*db,
            CallDirection::From,
            "NonExistent",
            None,
            None,
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
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls(
            &*db,
            CallDirection::From,
            "[invalid",
            None,
            None,
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
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls(
            &*db,
            CallDirection::From,
            "NonExistentModule",
            None,
            None,
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
