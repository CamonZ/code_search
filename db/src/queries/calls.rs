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

    // =========================================================================
    // Caller matching tests (line 63): use_regex && function_pattern.is_some()
    // Kills mutant: && replaced with ||
    // =========================================================================

    #[test]
    fn test_from_direction_exact_function_pattern_filters_caller() {
        // Tests line 63: when use_regex=false and function_pattern=Some("index"),
        // the exact match branch must be used. Asserts that only calls FROM
        // Controller.index are returned, not from show or create.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            Some("index"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            !calls.is_empty(),
            "Should find calls from Controller.index"
        );
        for call in &calls {
            assert_eq!(
                call.caller.name.as_ref(),
                "index",
                "All calls should be from the 'index' function, got '{}'",
                call.caller.name
            );
        }
    }

    #[test]
    fn test_from_direction_regex_with_no_function_pattern_returns_all_calls() {
        // Tests line 63: when use_regex=true and function_pattern=None,
        // the && condition is false, so no function filter is applied.
        // If mutated to ||, true || false = true would try to use
        // $function_pattern which is unbound, causing error or empty results.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            true,
            100,
        )
        .expect("Query should succeed with regex module and no function pattern");

        assert!(
            !calls.is_empty(),
            "Should return calls when regex=true but no function pattern is given"
        );
        // Verify we get calls from multiple different caller functions (no function filter)
        let caller_names: std::collections::HashSet<&str> =
            calls.iter().map(|c| c.caller.name.as_ref()).collect();
        assert!(
            caller_names.len() > 1,
            "Without function pattern, should return calls from multiple functions, got: {:?}",
            caller_names
        );
    }

    #[test]
    fn test_from_direction_regex_with_function_pattern_uses_regex_matching() {
        // Tests line 63: when use_regex=true AND function_pattern=Some(".*"),
        // the regex branch must be used (string::matches). This confirms
        // both conditions of the && must be true to enter the regex branch.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            Some("ind.*"),
            None,
            true,
            100,
        )
        .expect("Query should succeed with regex function pattern");

        assert!(
            !calls.is_empty(),
            "Regex pattern 'ind.*' should match 'index'"
        );
        for call in &calls {
            assert_eq!(
                call.caller.name.as_ref(),
                "index",
                "Regex 'ind.*' should only match 'index'"
            );
        }
    }

    // =========================================================================
    // Callee matching tests (line 86): use_regex && function_pattern.is_some()
    // Kills mutant: && replaced with ||
    // =========================================================================

    #[test]
    fn test_to_direction_exact_function_pattern_filters_callee() {
        // Tests line 86: when use_regex=false and function_pattern=Some("get_user"),
        // the exact match branch filters by callee function name.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::To,
            "MyApp.Accounts",
            Some("get_user"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            !calls.is_empty(),
            "Should find calls to Accounts.get_user"
        );
        for call in &calls {
            assert_eq!(
                call.callee.name.as_ref(),
                "get_user",
                "All calls should be to the 'get_user' function, got '{}'",
                call.callee.name
            );
        }
    }

    #[test]
    fn test_to_direction_regex_with_no_function_pattern_returns_all_calls() {
        // Tests line 86: when use_regex=true and function_pattern=None,
        // the && condition is false, so no function filter is applied.
        // If mutated to ||, true || false = true would try to use unbound
        // $function_pattern, causing error or empty results.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::To,
            "MyApp.Accounts",
            None,
            None,
            true,
            100,
        )
        .expect("Query should succeed with regex module and no function pattern");

        assert!(
            !calls.is_empty(),
            "Should return calls when regex=true but no function pattern is given"
        );
        // Verify we get calls to multiple different callee functions (no function filter)
        let callee_names: std::collections::HashSet<&str> =
            calls.iter().map(|c| c.callee.name.as_ref()).collect();
        assert!(
            callee_names.len() > 1,
            "Without function pattern, should return calls to multiple functions, got: {:?}",
            callee_names
        );
    }

    #[test]
    fn test_to_direction_regex_with_function_pattern_uses_regex_matching() {
        // Tests line 86: when use_regex=true AND function_pattern=Some("get_.*"),
        // the regex branch must be used. Confirms both conditions of && required.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::To,
            "MyApp.Accounts",
            Some("get_.*"),
            None,
            true,
            100,
        )
        .expect("Query should succeed with regex function pattern");

        assert!(
            !calls.is_empty(),
            "Regex pattern 'get_.*' should match 'get_user'"
        );
        for call in &calls {
            assert!(
                call.callee.name.as_ref().starts_with("get_"),
                "Regex 'get_.*' should only match functions starting with 'get_', got '{}'",
                call.callee.name
            );
        }
    }

    // =========================================================================
    // Arity filtering test (line 165): row.len() >= 13
    // Kills mutant: >= replaced with <
    // =========================================================================

    #[test]
    fn test_find_calls_parses_rows_with_exactly_13_columns() {
        // Tests line 165: rows with exactly 13 columns must be included.
        // The fixture data produces rows with exactly 13 columns.
        // If >= is mutated to <, all valid rows would be skipped, returning empty.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        // Controller has calls to Accounts.list_users, Accounts.get_user,
        // Service.process_request, Events.publish, and Notifier.send_email
        assert!(
            calls.len() >= 3,
            "Controller should have at least 3 outgoing calls, got {}",
            calls.len()
        );

        // Verify actual data was parsed from the rows (not just empty structs)
        for call in &calls {
            assert!(
                !call.caller.module.is_empty(),
                "Caller module should be parsed from row data"
            );
            assert!(
                !call.callee.name.is_empty(),
                "Callee function name should be parsed from row data"
            );
        }
    }

    // =========================================================================
    // Arity filtering boundary test (line 165): arity = $arity
    // Tests that arity filter correctly narrows results
    // =========================================================================

    #[test]
    fn test_find_calls_from_with_arity_filter() {
        // Tests arity filtering: Controller has index/2, show/2, create/2 (all arity 2).
        // With arity=2, all Controller calls should be returned.
        // With arity=1, no Controller calls should match (no arity-1 functions in Controller).
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls_arity_2 = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            Some(2),
            false,
            100,
        )
        .expect("Query with arity=2 should succeed");

        let calls_arity_1 = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            None,
            Some(1),
            false,
            100,
        )
        .expect("Query with arity=1 should succeed");

        assert!(
            !calls_arity_2.is_empty(),
            "Controller has arity-2 functions, should return calls"
        );
        // Controller.handle_event/1 is the only arity-1 function, verify it returns its calls
        // but there are fewer arity-1 calls than arity-2 calls
        assert!(
            calls_arity_2.len() > calls_arity_1.len(),
            "Arity filter should differentiate: arity=2 got {} calls, arity=1 got {} calls",
            calls_arity_2.len(),
            calls_arity_1.len()
        );
    }

    // =========================================================================
    // Position filtering tests (line 189):
    //   caller_start_line > 0 && caller_end_line > 0 && !caller_kind.is_empty()
    // Kills mutants:
    //   - > replaced with ==, <, >= on caller_start_line
    //   - > replaced with ==, <, >= on caller_end_line
    //   - && replaced with || (x2)
    //   - delete ! on caller_kind.is_empty()
    // =========================================================================

    #[test]
    fn test_calls_with_valid_clause_have_definition_info() {
        // Tests line 189: when caller_clause_id resolves to a valid clause,
        // caller_start_line > 0 AND caller_end_line > 0 AND kind is non-empty,
        // so with_definition is used. The caller should have kind, file,
        // start_line, and end_line populated.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            Some("index"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            !calls.is_empty(),
            "Should find calls from Controller.index"
        );

        let call = &calls[0];
        // The fixture has clause data for Controller.index at line 7,
        // so caller_start_line and caller_end_line should be > 0
        assert!(
            call.caller.start_line.is_some(),
            "Caller should have start_line from clause traversal"
        );
        assert!(
            call.caller.end_line.is_some(),
            "Caller should have end_line from clause traversal"
        );
        assert!(
            call.caller.kind.is_some(),
            "Caller should have kind from clause traversal"
        );
        assert!(
            call.caller.file.is_some(),
            "Caller should have file from clause traversal"
        );

        // Verify the values are positive (not zero defaults)
        assert!(
            call.caller.start_line.unwrap() > 0,
            "start_line should be positive, got {}",
            call.caller.start_line.unwrap()
        );
        assert!(
            call.caller.end_line.unwrap() > 0,
            "end_line should be positive, got {}",
            call.caller.end_line.unwrap()
        );
    }

    #[test]
    fn test_calls_without_clause_lack_definition_info() {
        // Tests line 189: when caller_clause_id points to a non-existent clause,
        // caller_start_line=0 and caller_end_line=0 and caller_kind="",
        // so FunctionRef::new is used instead of with_definition.
        // This kills the > vs == vs < vs >= mutants and the && vs || mutants
        // and the delete-! mutant, because the else branch must be taken.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Insert a call where the caller_clause_id points to a clause line
        // that doesn't exist (line 999), so the clause traversal returns NULLs
        let query = r#"
            RELATE
                functions:["MyApp.Repo", "get", 2]
                ->calls->
                functions:["MyApp.Repo", "all", 1]
            SET
                call_type = "local",
                caller_kind = "",
                file = "lib/my_app/repo.ex",
                line = 999,
                caller_clause_id = clauses:["MyApp.Repo", "get", 2, 999];
        "#;
        let params = crate::backend::QueryParams::new();
        db.execute_query(query, params)
            .expect("Insert test call should succeed");

        // Query for calls from Repo - should include the one at line 999
        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Repo",
            Some("get"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        // Find the call at line 999 (our test call with no valid clause)
        let no_clause_call = calls
            .iter()
            .find(|c| c.line == 999)
            .expect("Should find the call at line 999");

        // This call should NOT have definition info because the clause doesn't exist
        assert!(
            no_clause_call.caller.kind.is_none(),
            "Caller without valid clause should not have kind, got {:?}",
            no_clause_call.caller.kind
        );
        assert!(
            no_clause_call.caller.file.is_none(),
            "Caller without valid clause should not have file, got {:?}",
            no_clause_call.caller.file
        );
        assert!(
            no_clause_call.caller.start_line.is_none(),
            "Caller without valid clause should not have start_line, got {:?}",
            no_clause_call.caller.start_line
        );
        assert!(
            no_clause_call.caller.end_line.is_none(),
            "Caller without valid clause should not have end_line, got {:?}",
            no_clause_call.caller.end_line
        );
    }

    #[test]
    fn test_position_filter_start_line_zero_uses_minimal_ref() {
        // Tests line 189: when caller_start_line=0 but end_line>0 and kind non-empty,
        // the condition caller_start_line > 0 is false, so FunctionRef::new is used.
        // Kills: > replaced with == (0 == 0 would be true), >= (0 >= 0 true),
        // and && replaced with || (false || true || true = true).
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Create a clause with start_line=0 (the edge case)
        let create_clause = r#"
            CREATE clauses:["MyApp.Metrics", "increment", 1, 50] SET
                module_name = "MyApp.Metrics",
                function_name = "increment",
                arity = 1,
                line = 50,
                source_file = "lib/my_app/metrics.ex",
                source_file_absolute = "",
                kind = "def",
                start_line = 0,
                end_line = 50,
                pattern = "",
                guard = NONE,
                source_sha = "",
                ast_sha = "",
                complexity = 1,
                max_nesting_depth = 1,
                generated_by = NONE,
                macro_source = NONE;
        "#;
        db.execute_query(create_clause, crate::backend::QueryParams::new())
            .expect("Create clause should succeed");

        // Create a call that references this clause
        let create_call = r#"
            RELATE
                functions:["MyApp.Metrics", "increment", 1]
                ->calls->
                functions:["MyApp.Logger", "debug", 1]
            SET
                call_type = "remote",
                caller_kind = "def",
                file = "lib/my_app/metrics.ex",
                line = 50,
                caller_clause_id = clauses:["MyApp.Metrics", "increment", 1, 50];
        "#;
        db.execute_query(create_call, crate::backend::QueryParams::new())
            .expect("Create call should succeed");

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Metrics",
            Some("increment"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        let test_call = calls
            .iter()
            .find(|c| c.line == 50)
            .expect("Should find call at line 50");

        // start_line=0 means the condition fails -> FunctionRef::new (no definition)
        assert!(
            test_call.caller.start_line.is_none(),
            "start_line=0 in clause should result in no definition info, got {:?}",
            test_call.caller.start_line
        );
        assert!(
            test_call.caller.kind.is_none(),
            "When start_line=0, should use FunctionRef::new (no kind), got {:?}",
            test_call.caller.kind
        );
    }

    #[test]
    fn test_position_filter_end_line_zero_uses_minimal_ref() {
        // Tests line 189: when caller_end_line=0 but start_line>0 and kind non-empty,
        // the condition caller_end_line > 0 is false, so FunctionRef::new is used.
        // Kills: > replaced with == on end_line, and && replaced with ||.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Create a clause with end_line=0 (the edge case)
        let create_clause = r#"
            CREATE clauses:["MyApp.Metrics", "record", 2, 50] SET
                module_name = "MyApp.Metrics",
                function_name = "record",
                arity = 2,
                line = 50,
                source_file = "lib/my_app/metrics.ex",
                source_file_absolute = "",
                kind = "def",
                start_line = 50,
                end_line = 0,
                pattern = "",
                guard = NONE,
                source_sha = "",
                ast_sha = "",
                complexity = 1,
                max_nesting_depth = 1,
                generated_by = NONE,
                macro_source = NONE;
        "#;
        db.execute_query(create_clause, crate::backend::QueryParams::new())
            .expect("Create clause should succeed");

        // Create a call that references this clause
        let create_call = r#"
            RELATE
                functions:["MyApp.Metrics", "record", 2]
                ->calls->
                functions:["MyApp.Logger", "debug", 1]
            SET
                call_type = "remote",
                caller_kind = "def",
                file = "lib/my_app/metrics.ex",
                line = 50,
                caller_clause_id = clauses:["MyApp.Metrics", "record", 2, 50];
        "#;
        db.execute_query(create_call, crate::backend::QueryParams::new())
            .expect("Create call should succeed");

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Metrics",
            Some("record"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        let test_call = calls
            .iter()
            .find(|c| c.line == 50 && c.callee.name.as_ref() == "debug")
            .expect("Should find test call at line 50 to debug");

        // end_line=0 means the condition fails -> FunctionRef::new (no definition)
        assert!(
            test_call.caller.end_line.is_none(),
            "end_line=0 in clause should result in no definition info, got {:?}",
            test_call.caller.end_line
        );
        assert!(
            test_call.caller.kind.is_none(),
            "When end_line=0, should use FunctionRef::new (no kind), got {:?}",
            test_call.caller.kind
        );
    }

    #[test]
    fn test_position_filter_empty_kind_uses_minimal_ref() {
        // Tests line 189: when caller_kind="" but start_line>0 and end_line>0,
        // the condition !caller_kind.is_empty() is false, so FunctionRef::new is used.
        // Kills: delete ! on is_empty() (without !, non-empty kind passes -> wrong branch).
        //
        // caller_kind comes from in.kind (the function record), so we must create
        // a function with kind="" to trigger this path.
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Create a module for isolation
        let create_mod = r#"CREATE modules:["TestMod.EmptyKind"] SET name = "TestMod.EmptyKind", file = "", source = "unknown";"#;
        db.execute_query(create_mod, crate::backend::QueryParams::new())
            .expect("Create module should succeed");

        // Create a function with kind="" (the edge case for line 189)
        let create_fn = r#"
            CREATE functions:["TestMod.EmptyKind", "no_kind_fn", 0] SET
                module_name = "TestMod.EmptyKind",
                name = "no_kind_fn",
                arity = 0,
                kind = "",
                file = "lib/test_mod.ex",
                start_line = 10;
        "#;
        db.execute_query(create_fn, crate::backend::QueryParams::new())
            .expect("Create function should succeed");

        // Create a callee function
        let create_callee = r#"
            CREATE functions:["TestMod.EmptyKind", "target_fn", 0] SET
                module_name = "TestMod.EmptyKind",
                name = "target_fn",
                arity = 0,
                kind = "def",
                file = "lib/test_mod.ex",
                start_line = 20;
        "#;
        db.execute_query(create_callee, crate::backend::QueryParams::new())
            .expect("Create callee function should succeed");

        // Create a clause with valid start_line and end_line but the function has kind=""
        let create_clause = r#"
            CREATE clauses:["TestMod.EmptyKind", "no_kind_fn", 0, 10] SET
                module_name = "TestMod.EmptyKind",
                function_name = "no_kind_fn",
                arity = 0,
                line = 10,
                source_file = "lib/test_mod.ex",
                source_file_absolute = "",
                kind = "",
                start_line = 10,
                end_line = 20,
                pattern = "",
                guard = NONE,
                source_sha = "",
                ast_sha = "",
                complexity = 1,
                max_nesting_depth = 1,
                generated_by = NONE,
                macro_source = NONE;
        "#;
        db.execute_query(create_clause, crate::backend::QueryParams::new())
            .expect("Create clause should succeed");

        // Create a call where caller has kind="" but clause has valid start/end lines
        let create_call = r#"
            RELATE
                functions:["TestMod.EmptyKind", "no_kind_fn", 0]
                ->calls->
                functions:["TestMod.EmptyKind", "target_fn", 0]
            SET
                call_type = "local",
                caller_kind = "",
                file = "lib/test_mod.ex",
                line = 10,
                caller_clause_id = clauses:["TestMod.EmptyKind", "no_kind_fn", 0, 10];
        "#;
        db.execute_query(create_call, crate::backend::QueryParams::new())
            .expect("Create call should succeed");

        let calls = find_calls(
            &*db,
            CallDirection::From,
            "TestMod.EmptyKind",
            Some("no_kind_fn"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            !calls.is_empty(),
            "Should find the call from no_kind_fn"
        );
        let test_call = &calls[0];

        // caller_kind="" (from in.kind on function record) means
        // !caller_kind.is_empty() is false -> FunctionRef::new
        assert!(
            test_call.caller.kind.is_none(),
            "Empty kind on function should result in no definition info, got {:?}",
            test_call.caller.kind
        );
        // But the caller still has module/name/arity from the basic FunctionRef::new
        assert_eq!(
            test_call.caller.module.as_ref(),
            "TestMod.EmptyKind",
            "Caller module should still be set"
        );
        assert_eq!(
            test_call.caller.name.as_ref(),
            "no_kind_fn",
            "Caller name should still be set"
        );
    }

    #[test]
    fn test_position_filter_all_valid_uses_full_definition() {
        // Tests line 189: when all three conditions are true (start_line > 0,
        // end_line > 0, kind non-empty), with_definition is used.
        // This is the complementary test to the zero/empty tests above.
        // Together they kill && -> || mutants because:
        //   - start_line=0 test: false && true && true = false (correct)
        //     but false || true || true = true (wrong, would use with_definition)
        //   - This test: true && true && true = true (correct, uses with_definition)
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Controller.index/2 calls Accounts.list_users/0 at line 7
        // The clause at line 7 has kind="def", start_line=7, end_line=7
        let calls = find_calls(
            &*db,
            CallDirection::From,
            "MyApp.Controller",
            Some("show"),
            None,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(!calls.is_empty(), "Should find calls from Controller.show");

        let call = &calls[0];
        // All three conditions true -> with_definition path
        assert!(
            call.caller.kind.is_some(),
            "Valid clause should produce kind via with_definition"
        );
        assert!(
            call.caller.file.is_some(),
            "Valid clause should produce file via with_definition"
        );
        assert!(
            call.caller.start_line.is_some() && call.caller.start_line.unwrap() > 0,
            "Valid clause should produce positive start_line"
        );
        assert!(
            call.caller.end_line.is_some() && call.caller.end_line.unwrap() > 0,
            "Valid clause should produce positive end_line"
        );
        assert!(
            !call.caller.kind.as_ref().unwrap().is_empty(),
            "Kind should be non-empty for valid clause"
        );
    }
}
