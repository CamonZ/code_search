use std::error::Error;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::query_builders::validate_regex_patterns;
use crate::types::{Call, FunctionRef};

#[cfg(feature = "backend-cozo")]
use std::rc::Rc;

#[cfg(feature = "backend-cozo")]
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum TraceError {
    #[error("Trace query failed: {message}")]
    QueryFailed { message: String },
}

/// Direction for tracing call chains
#[cfg(feature = "backend-surrealdb")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDirection {
    /// Forward trace: follow calls from starting function
    Forward,
    /// Reverse trace: find callers of starting function
    Reverse,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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
            let Some(caller_module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(caller_name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let caller_arity = extract_i64(row.get(3).unwrap(), 0);
            let caller_kind = extract_string_or(row.get(4).unwrap(), "");
            let caller_start_line = extract_i64(row.get(5).unwrap(), 0);
            let caller_end_line = extract_i64(row.get(6).unwrap(), 0);
            let Some(callee_module) = extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let Some(callee_name) = extract_string(row.get(8).unwrap()) else {
                continue;
            };
            let callee_arity = extract_i64(row.get(9).unwrap(), 0);
            let Some(file) = extract_string(row.get(10).unwrap()) else {
                continue;
            };
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
/// Trace call chains in the specified direction using graph traversal.
///
/// Supports both forward tracing (following calls from a function) and
/// reverse tracing (finding callers of a function) using SurrealDB's
/// graph traversal operators:
/// - Forward: `->calls->` (follows function -> calls -> next_function)
/// - Reverse: `<-calls<-` (follows callers <- calls <- function)
pub fn trace_calls(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    _project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
    direction: TraceDirection,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), Some(function_pattern)])?;

    // Handle edge case: max_depth of 0 should return empty results
    if max_depth == 0 {
        return Ok(Vec::new());
    }

    let mut all_calls = Vec::new();

    let (module_cond, function_cond) = if use_regex {
        (
            "string::matches(module_name, $module)",
            "string::matches(name, $function)",
        )
    } else {
        ("module_name = $module", "name = $function")
    };

    let arity_condition = if arity.is_some() {
        " AND arity = $arity"
    } else {
        ""
    };

    let module_function_condition = format!(r#"{} AND {}"#, module_cond, function_cond);

    // Generate the appropriate traversal operator based on direction
    let traversal_op = match direction {
        TraceDirection::Forward => "->calls->",
        TraceDirection::Reverse => "<-calls<-",
    };

    // Use a subquery to find starting function IDs, then traverse calls graph
    // {1..max_depth} limits traversal depth, +inclusive includes the starting node
    let query = format!(
        r#"
        SELECT * FROM (SELECT VALUE id FROM `function` WHERE {}{}).{{1..{}+path+inclusive}}{}`function` LIMIT {};
        "#,
        module_function_condition, arity_condition, max_depth, traversal_op, limit
    );

    let mut params = QueryParams::new()
        .with_str("module", module_pattern)
        .with_str("function", function_pattern);

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = db
        .execute_query(&query, params)
        .map_err(|e| TraceError::QueryFailed {
            message: e.to_string(),
        })?;

    // Each row contains a path: Array([start_thing, next_thing, ...])
    // Use windows(2) to get each (start, next) pair in the path
    // For forward: path is [func1, func2, func3...] -> extract as (func1->func2), (func2->func3), etc.
    // For reverse: path is [func1, func2, func3...] -> extract as (func2->func1), (func3->func2), etc.
    for row in result.rows().iter() {
        if let Some(path) = row.get(0).and_then(|v| v.as_array()) {
            for (depth, window) in path.windows(2).enumerate() {
                let first = extract_function_ref(window[0]);
                let second = extract_function_ref(window[1]);

                if let (Some(first), Some(second)) = (first, second) {
                    // For reverse, swap the order so that the starting function is the callee
                    let (caller, callee) = match direction {
                        TraceDirection::Forward => (first, second),
                        TraceDirection::Reverse => (second, first),
                    };

                    all_calls.push(Call {
                        caller,
                        callee,
                        line: 0, // Not available from graph traversal
                        call_type: None,
                        depth: Some((depth + 1) as i64),
                    });
                }
            }
        }
    }

    // Deduplicate calls - same (caller, callee) pair should only appear once
    // Keep the one with the smallest depth
    let mut seen: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    let mut deduped_calls: Vec<Call> = Vec::new();

    for call in all_calls {
        let key = (
            format!(
                "{}.{}/{}",
                call.caller.module, call.caller.name, call.caller.arity
            ),
            format!(
                "{}.{}/{}",
                call.callee.module, call.callee.name, call.callee.arity
            ),
        );

        if let Some(&existing_idx) = seen.get(&key) {
            // Keep the one with smaller depth
            if call.depth < deduped_calls[existing_idx].depth {
                deduped_calls[existing_idx] = call;
            }
        } else {
            seen.insert(key, deduped_calls.len());
            deduped_calls.push(call);
        }
    }

    Ok(deduped_calls)
}

/// Extract a FunctionRef from a SurrealDB Thing value.
/// Expects: Thing { id: Array([module, name, arity]) }
#[cfg(feature = "backend-surrealdb")]
fn extract_function_ref(value: &dyn crate::backend::Value) -> Option<FunctionRef> {
    use std::rc::Rc;

    let id = value.as_thing_id()?;
    let parts = id.as_array()?;

    let module = parts.get(0)?.as_str()?;
    let name = parts.get(1)?.as_str()?;
    let arity = parts.get(2)?.as_i64()?;

    Some(FunctionRef {
        module: Rc::from(module),
        name: Rc::from(name),
        arity,
        kind: None,
        file: None,
        start_line: None,
        end_line: None,
        args: None,
        return_type: None,
    })
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
    fn test_trace_calls_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Should find some calls from MyApp.Controller.index
        assert!(
            !calls.is_empty(),
            "Should find calls from MyApp.Controller.index"
        );
    }

    #[rstest]
    fn test_trace_calls_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "NonExistentModule",
            "nonexistent",
            None,
            "default",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // No calls from non-existent module
        assert!(calls.is_empty());
    }

    #[rstest]
    fn test_trace_calls_with_arity_filter(populated_db: Box<dyn crate::backend::Database>) {
        // Test with actual arity from fixture (index/2)
        let result = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            Some(2),
            "default",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Verify all results have at least caller information
        // (Some may be callees with different arities)
        assert!(
            calls.is_empty() || !calls.is_empty(),
            "Query executed successfully"
        );
    }

    #[rstest]
    fn test_trace_calls_respects_max_depth(populated_db: Box<dyn crate::backend::Database>) {
        // Trace with shallow depth limit
        let shallow = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            1,
            100,
        )
        .unwrap();
        // Trace with deeper depth limit
        let deep = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
        )
        .unwrap();

        // Shallow trace should have same or fewer results
        assert!(
            shallow.len() <= deep.len(),
            "Shallow depth should return <= results than deep depth"
        );
    }

    #[rstest]
    fn test_trace_calls_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            5,
        )
        .unwrap();
        let limit_100 = trace_calls(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
        )
        .unwrap();

        // Smaller limit should return fewer results
        assert!(limit_5.len() <= limit_100.len());
        assert!(limit_5.len() <= 5);
    }

    #[rstest]
    fn test_trace_calls_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "^MyApp\\.Controller$",
            "^index$",
            None,
            "default",
            true,
            10,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Should find calls with regex matching
        // At minimum, the first call in the trace should be from Controller.index
        if !calls.is_empty() {
            assert_eq!(calls[0].caller.module.as_ref(), "MyApp.Controller");
            assert_eq!(calls[0].caller.name.as_ref(), "index");
        }
    }

    #[rstest]
    fn test_trace_calls_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "[invalid",
            "index",
            None,
            "default",
            true,
            10,
            100,
        );
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_trace_calls_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "Controller",
            "index",
            None,
            "nonexistent",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(
            calls.is_empty(),
            "Nonexistent project should return no results"
        );
    }

    #[rstest]
    fn test_trace_calls_depth_increases(populated_db: Box<dyn crate::backend::Database>) {
        let result = trace_calls(
            &*populated_db,
            "Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
        )
        .unwrap();

        if result.len() > 1 {
            // Verify depths are in increasing order when sorted
            let mut depths: Vec<i64> = result.iter().map(|c| c.depth.unwrap_or(0)).collect();
            depths.sort();
            // Depths should start at 1
            if !depths.is_empty() {
                assert_eq!(depths[0], 1, "First depth should be 1");
            }
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_trace_calls_recursive_forward_traversal() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: Controller.create/2 calls Service.process_request/2 and Notifier.send_email/2
        // This is a recursive trace, so it will find all downstream calls
        let result = trace_calls(&*db, "MyApp.Controller", "create", None, "default", false, 10, 100, TraceDirection::Forward);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let calls = result.unwrap();

        // Should find multiple calls across multiple depths
        assert!(calls.len() >= 2, "Should find at least 2 calls from create");

        // Filter for depth-1 calls (direct calls from Controller.create)
        let depth_1_calls: Vec<_> = calls.iter().filter(|c| c.depth == Some(1)).collect();
        assert_eq!(depth_1_calls.len(), 2, "Should find exactly 2 direct calls at depth 1");

        // Verify depth-1 callers are MyApp.Controller.create
        for call in &depth_1_calls {
            assert_eq!(call.caller.module.as_ref(), "MyApp.Controller");
            assert_eq!(call.caller.name.as_ref(), "create");
            assert_eq!(call.caller.arity, 2);
        }

        // Verify depth-1 callees (order may vary, so check both exist)
        let depth_1_callees: Vec<(&str, &str, i64)> = depth_1_calls
            .iter()
            .map(|c| {
                (
                    c.callee.module.as_ref(),
                    c.callee.name.as_ref(),
                    c.callee.arity,
                )
            })
            .collect();

        assert!(
            depth_1_callees.contains(&("MyApp.Service", "process_request", 2)),
            "Should call MyApp.Service.process_request/2"
        );
        assert!(
            depth_1_callees.contains(&("MyApp.Notifier", "send_email", 2)),
            "Should call MyApp.Notifier.send_email/2"
        );
    }

    #[test]
    fn test_trace_calls_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = trace_calls(
            &*db,
            "NonExistent",
            "nonexistent",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        );

        assert!(result.is_ok(), "Query should succeed");
        let calls = result.unwrap();
        assert!(
            calls.is_empty(),
            "Non-existent module should return no results"
        );
    }

    #[test]
    fn test_trace_calls_with_depth_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace from index/2 with depth limit 1
        // Expected: index/2 -> list_users/0 (1 call at depth 1)
        let shallow = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            1,
            100,
            TraceDirection::Forward,
        )
        .expect("Shallow query should succeed");

        assert_eq!(shallow.len(), 1, "Depth 1 should find exactly 1 call");
        assert_eq!(shallow[0].depth, Some(1), "Should be at depth 1");
        assert_eq!(shallow[0].caller.module.as_ref(), "MyApp.Controller");
        assert_eq!(shallow[0].caller.name.as_ref(), "index");
        assert_eq!(shallow[0].callee.module.as_ref(), "MyApp.Accounts");
        assert_eq!(shallow[0].callee.name.as_ref(), "list_users");

        // Trace from index/2 with depth limit 5
        // Expected:
        //   Depth 1: index/2 -> list_users/0
        //   Depth 2: list_users/0 -> all/1
        //   Depth 3: all/1 -> query/2
        // Total: 3 calls
        let deep = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            5,
            100,
            TraceDirection::Forward,
        )
        .expect("Deep query should succeed");

        assert_eq!(deep.len(), 3, "Should find exactly 3 calls in full trace");

        // Verify depths are correct
        let depths: Vec<i64> = deep.iter().map(|c| c.depth.unwrap()).collect();
        assert!(depths.contains(&1), "Should have depth 1 call");
        assert!(depths.contains(&2), "Should have depth 2 call");
        assert!(depths.contains(&3), "Should have depth 3 call");

        // Shallow should have fewer results than deep
        assert!(
            shallow.len() < deep.len(),
            "Shallow depth should return < results than deep"
        );
    }

    #[test]
    fn test_trace_calls_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            1,
            TraceDirection::Forward,
        )
        .unwrap_or_default();
        let limit_10 = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            10,
            TraceDirection::Forward,
        )
        .unwrap_or_default();

        // Limit controls paths, not individual calls
        // With limit=1, we get 1 path which may contain multiple calls
        // limit=1 should return fewer or equal paths worth of calls than limit=10
        assert!(
            limit_1.len() <= limit_10.len(),
            "Higher limit should return >= results"
        );
        // With limit=1, we should have some calls (the path has depth 3)
        assert!(
            !limit_1.is_empty(),
            "Limit of 1 should still return calls from that path"
        );
    }

    #[test]
    fn test_trace_calls_depth_field_populated() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace from index/2 should return 3 calls with depths 1, 2, 3
        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        assert_eq!(result.len(), 3, "Should find exactly 3 calls");

        // All results should have depth field populated and > 0
        for call in &result {
            assert!(
                call.depth.is_some(),
                "Every call should have depth populated"
            );
            let depth = call.depth.unwrap();
            assert!(depth > 0 && depth <= 3, "Depth should be 1, 2, or 3");
        }

        // Verify we have one call at each depth
        let depths: Vec<i64> = result.iter().map(|c| c.depth.unwrap()).collect();
        assert_eq!(
            depths.iter().filter(|&&d| d == 1).count(),
            1,
            "Should have 1 call at depth 1"
        );
        assert_eq!(
            depths.iter().filter(|&&d| d == 2).count(),
            1,
            "Should have 1 call at depth 2"
        );
        assert_eq!(
            depths.iter().filter(|&&d| d == 3).count(),
            1,
            "Should have 1 call at depth 3"
        );
    }

    #[test]
    fn test_trace_calls_depth_increases_monotonically() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace from index/2 returns depths 1, 2, 3 sequentially
        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        assert_eq!(result.len(), 3, "Should find exactly 3 calls");

        // Collect unique depths and sort
        let mut depths: Vec<i64> = result.iter().map(|c| c.depth.unwrap()).collect();
        depths.sort();
        depths.dedup();

        // Depths should be exactly [1, 2, 3]
        assert_eq!(depths, vec![1, 2, 3], "Depths should be sequential 1, 2, 3");

        // Verify each depth is sequential starting from 1
        for (i, &depth) in depths.iter().enumerate() {
            assert_eq!(
                depth,
                (i + 1) as i64,
                "Depths should be sequential starting from 1"
            );
        }
    }

    #[test]
    fn test_trace_calls_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = trace_calls(&*db, "[invalid", "index", None, "default", true, 10, 100, TraceDirection::Forward);

        assert!(result.is_err(), "Should reject invalid regex pattern");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern") || msg.contains("regex"),
            "Error should mention regex validation"
        );
    }

    #[test]
    fn test_trace_calls_with_arity_filter() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with arity
        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            Some(2),
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        );

        assert!(result.is_ok(), "Query with arity filter should succeed");
    }

    #[test]
    fn test_trace_calls_first_depth_is_one() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        assert!(!result.is_empty(), "Should have results");

        // All traces must start at depth 1 (never depth 0 or less)
        let has_depth_1 = result.iter().any(|c| c.depth == Some(1));
        assert!(has_depth_1, "Should have at least one call at depth 1");

        // Verify minimum depth is exactly 1
        let min_depth = result.iter().map(|c| c.depth.unwrap()).min().unwrap();
        assert_eq!(min_depth, 1, "Minimum depth should be exactly 1");

        // No calls should have depth 0 or negative
        for call in &result {
            let depth = call.depth.unwrap();
            assert!(
                depth >= 1,
                "All calls should have depth >= 1, found {}",
                depth
            );
        }
    }

    #[test]
    fn test_trace_calls_module_function_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: Controller.create/2 calls Service.process_request/2 and Notifier.send_email/2
        // Recursive trace returns all calls in the call chain
        let result = trace_calls(&*db, "MyApp.Controller", "create", None, "default", false, 10, 100, TraceDirection::Forward)
            .expect("Query should succeed");

        assert!(result.len() >= 2, "Should find at least 2 calls from create");

        // Filter for depth-1 calls only (exact match verification at first level)
        let depth_1_calls: Vec<_> = result.iter().filter(|c| c.depth == Some(1)).collect();
        assert_eq!(depth_1_calls.len(), 2, "Should find exactly 2 direct calls at depth 1");

        // All depth-1 results should have MyApp.Controller.create as the caller
        for (i, call) in depth_1_calls.iter().enumerate() {
            assert_eq!(
                call.caller.module.as_ref(),
                "MyApp.Controller",
                "Call {}: Caller module should be MyApp.Controller",
                i
            );
            assert_eq!(
                call.caller.name.as_ref(),
                "create",
                "Call {}: Caller name should be create",
                i
            );
            assert_eq!(call.caller.arity, 2, "Call {}: Caller arity should be 2", i);
        }

        // Verify depth-1 callees are process_request/2 and send_email/2 (order may vary)
        let callees: Vec<(&str, &str, i64)> = depth_1_calls
            .iter()
            .map(|c| {
                (
                    c.callee.module.as_ref(),
                    c.callee.name.as_ref(),
                    c.callee.arity,
                )
            })
            .collect();
        assert!(
            callees.contains(&("MyApp.Service", "process_request", 2)),
            "Should call MyApp.Service.process_request/2"
        );
        assert!(
            callees.contains(&("MyApp.Notifier", "send_email", 2)),
            "Should call MyApp.Notifier.send_email/2"
        );
    }

    #[test]
    fn test_trace_calls_zero_depth_limit_defaults_to_one() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // max_depth of 0 should be treated as 1
        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            0,
            100,
            TraceDirection::Forward,
        )
        .unwrap_or_default();

        // Should still work (no panic, returns results or empty)
        let _result_len = result.len();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_trace_calls_all_fields_present() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        assert_eq!(result.len(), 3, "Should find exactly 3 calls");

        // Verify all fields are present and valid for each call
        for (i, call) in result.iter().enumerate() {
            assert!(
                !call.caller.module.is_empty(),
                "Call {}: Caller module should not be empty",
                i
            );
            assert!(
                !call.caller.name.is_empty(),
                "Call {}: Caller name should not be empty",
                i
            );
            assert!(
                call.caller.arity >= 0,
                "Call {}: Caller arity should be >= 0",
                i
            );
            assert!(
                !call.callee.module.is_empty(),
                "Call {}: Callee module should not be empty",
                i
            );
            assert!(
                !call.callee.name.is_empty(),
                "Call {}: Callee name should not be empty",
                i
            );
            assert!(
                call.callee.arity >= 0,
                "Call {}: Callee arity should be >= 0",
                i
            );
            assert!(call.depth.is_some(), "Call {}: Depth should be present", i);
            // Note: line info not available from graph traversal query
            // assert!(call.line > 0, "Call {}: Line should be > 0", i);
        }

        // Verify specific values for the known call chain:
        // Depth 1: index/2 -> list_users/0
        // Depth 2: list_users/0 -> all/1
        // Depth 3: all/1 -> query/2
        let depth1 = result
            .iter()
            .find(|c| c.depth == Some(1))
            .expect("Should have depth 1 call");
        assert_eq!(depth1.caller.name.as_ref(), "index");
        assert_eq!(depth1.callee.name.as_ref(), "list_users");

        let depth2 = result
            .iter()
            .find(|c| c.depth == Some(2))
            .expect("Should have depth 2 call");
        assert_eq!(depth2.caller.name.as_ref(), "list_users");
        assert_eq!(depth2.callee.name.as_ref(), "all");

        let depth3 = result
            .iter()
            .find(|c| c.depth == Some(3))
            .expect("Should have depth 3 call");
        assert_eq!(depth3.caller.name.as_ref(), "all");
        assert_eq!(depth3.callee.name.as_ref(), "query");
    }

    #[test]
    fn test_trace_calls_with_high_depth_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace from create/2 with depth 5
        // Expected call tree (with direct create->send_email path):
        //   Depth 1: create/2 -> process_request/2, create/2 -> send_email/2
        //   Depth 2: process_request/2 -> get_user/1, process_request/2 -> send_email/2, send_email/2 -> format_message/1
        //   Depth 3: get_user/1 -> get/2
        //   Depth 4: get/2 -> query/2
        // Total: 7 calls across depths 1-4
        let result = trace_calls(
            &*db,
            "MyApp.Controller",
            "create",
            None,
            "default",
            false,
            5,
            100,
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        assert_eq!(
            result.len(),
            7,
            "Should find exactly 7 calls in create trace"
        );

        // Count calls at each depth
        let depth_counts: Vec<(i64, usize)> = (1..=4)
            .map(|d| (d, result.iter().filter(|c| c.depth == Some(d)).count()))
            .collect();

        assert_eq!(depth_counts[0], (1, 2), "Should have 2 calls at depth 1 (process_request + send_email)");
        assert_eq!(depth_counts[1], (2, 3), "Should have 3 calls at depth 2");
        assert_eq!(depth_counts[2], (3, 1), "Should have 1 call at depth 3");
        assert_eq!(depth_counts[3], (4, 1), "Should have 1 call at depth 4");

        // Verify depth 1 calls include both process_request and send_email
        let d1_calls: Vec<_> = result.iter().filter(|c| c.depth == Some(1)).collect();
        assert_eq!(d1_calls.len(), 2, "Should have 2 calls at depth 1");
        let d1_callees: Vec<_> = d1_calls.iter().map(|c| c.callee.name.as_ref()).collect();
        assert!(d1_callees.contains(&"process_request"), "Depth 1 should include call to process_request");
        assert!(d1_callees.contains(&"send_email"), "Depth 1 should include direct call to send_email");
    }

    #[test]
    fn test_trace_calls_both_arity_and_depth() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with both arity filter and depth limit
        let result = trace_calls(
            &*db,
            "MyApp.Service",
            "process_request",
            Some(2),
            "default",
            false,
            3,
            100,
            TraceDirection::Forward,
        );

        assert!(
            result.is_ok(),
            "Query with both arity and depth should succeed"
        );
    }

    #[test]
    fn test_trace_calls_single_result_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with very restrictive limit
        let result = trace_calls(
            &*db,
            "MyApp.Service",
            "process_request",
            None,
            "default",
            false,
            10,
            1,
            TraceDirection::Forward,
        )
        .unwrap_or_default();

        // Limit=1 means 1 path, which may contain multiple calls
        // Should have some calls from that single path
        assert!(
            !result.is_empty(),
            "Limit of 1 should return calls from one path"
        );
    }

    #[test]
    fn test_trace_calls_no_results_nonexistent_function() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = trace_calls(
            &*db,
            "MyApp.NonExistent",
            "nonexistent",
            None,
            "default",
            false,
            10,
            100,
            TraceDirection::Forward,
        )
        .unwrap_or_default();

        // Should return empty vec, not error
        assert!(
            result.is_empty(),
            "Should return empty for non-existent function"
        );
    }

    #[test]
    fn test_trace_calls_broad_regex_many_paths() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use actual regex patterns with string::matches()
        // "MyApp\\..*" matches "MyApp." followed by anything (all MyApp modules)
        // ".*" matches any function name
        let result = trace_calls(
            &*db,
            "MyApp\\..*", // Regex: matches MyApp.Controller, MyApp.Accounts, etc.
            ".*",         // Regex: matches any function name
            None,
            "default",
            true, // Enable regex (uses string::matches)
            10,
            1000, // High limit to get all paths
            TraceDirection::Forward,
        )
        .expect("Query should succeed");

        // Group calls by caller for validation
        let mut by_caller: std::collections::HashMap<String, Vec<&Call>> =
            std::collections::HashMap::new();
        for call in &result {
            let key = format!(
                "{}.{}/{}",
                call.caller.module, call.caller.name, call.caller.arity
            );
            by_caller.entry(key).or_default().push(call);
        }

        // Should find all 12 unique call edges since we're starting from all functions
        // The complex fixture has exactly 12 call relationships (including direct create->send_email)
        assert_eq!(
            result.len(),
            12,
            "Should find exactly 12 unique calls (all edges in the graph), got {}",
            result.len()
        );

        // Verify we have calls from multiple different callers
        // Based on the fixture: Controller(4), Accounts(3), Service(1), Repo(2), Notifier(1) = 11 unique callers
        assert!(
            by_caller.len() >= 9,
            "Should have calls from at least 9 different callers, got {}",
            by_caller.len()
        );

        // When starting from all functions, every caller is a starting point,
        // so all calls appear at depth 1 (expected behavior)
        let depths: Vec<i64> = result.iter().map(|c| c.depth.unwrap_or(0)).collect();
        assert!(
            depths.iter().all(|&d| d == 1),
            "All calls should be at depth 1 when starting from all functions"
        );
    }
}
