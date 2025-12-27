use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::Database;

#[cfg(feature = "backend-cozo")]
use crate::backend::QueryParams;
#[cfg(feature = "backend-cozo")]
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[cfg(feature = "backend-surrealdb")]
use crate::queries::trace::TraceDirection;

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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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
    // Use trace_calls with Reverse direction
    let calls = crate::queries::trace::trace_calls(
        db,
        module_pattern,
        function_pattern,
        arity,
        project,
        use_regex,
        max_depth,
        limit,
        TraceDirection::Reverse,
    )?;

    // Convert Call results to ReverseTraceStep
    let steps = calls
        .into_iter()
        .map(|call| ReverseTraceStep {
            depth: call.depth.unwrap_or(0),
            caller_module: call.caller.module.to_string(),
            caller_function: call.caller.name.to_string(),
            caller_arity: call.caller.arity,
            caller_kind: call.caller.kind.map(|k| k.to_string()).unwrap_or_default(),
            caller_start_line: call.caller.start_line.unwrap_or(0),
            caller_end_line: call.caller.end_line.unwrap_or(0),
            callee_module: call.callee.module.to_string(),
            callee_function: call.callee.name.to_string(),
            callee_arity: call.callee.arity,
            file: String::new(), // Not available from SurrealDB graph traversal
            line: call.line,
        })
        .collect();

    Ok(steps)
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::call_graph_db("default")
    }

    #[rstest]
    fn test_reverse_trace_calls_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 10, 100);
        assert!(result.is_ok());
        let steps = result.unwrap();
        // Should find some callers to Accounts.get_user
        assert!(!steps.is_empty(), "Should find callers to MyApp.Accounts.get_user");
    }

    #[rstest]
    fn test_reverse_trace_calls_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(
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
        let steps = result.unwrap();
        // No callers to non-existent function
        assert!(steps.is_empty());
    }

    #[rstest]
    fn test_reverse_trace_calls_with_arity_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(
            &*populated_db,
            "MyApp.Accounts",
            "get_user",
            Some(1),
            "default",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let steps = result.unwrap();
        // Verify all results have the specified callee arity
        for step in &steps {
            assert_eq!(
                step.callee_arity, 1,
                "All calls should target callee with arity 1"
            );
        }
    }

    #[rstest]
    fn test_reverse_trace_calls_respects_max_depth(populated_db: Box<dyn crate::backend::Database>) {
        // Trace with shallow depth limit
        let shallow = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 1, 100)
            .unwrap();
        // Trace with deeper depth limit
        let deep = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 10, 100)
            .unwrap();

        // Shallow trace should have same or fewer results
        assert!(shallow.len() <= deep.len(), "Shallow depth should return <= results than deep depth");
    }

    #[rstest]
    fn test_reverse_trace_calls_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 10, 5)
            .unwrap();
        let limit_100 = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 10, 100)
            .unwrap();

        // Smaller limit should return fewer results
        assert!(limit_5.len() <= limit_100.len());
        assert!(limit_5.len() <= 5);
    }

    #[rstest]
    fn test_reverse_trace_calls_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(
            &*populated_db,
            "^MyApp\\.Accounts$",
            "^get_user$",
            None,
            "default",
            true,
            10,
            100,
        );
        assert!(result.is_ok());
        let steps = result.unwrap();
        // Should find calls with regex matching
        for step in &steps {
            assert_eq!(step.callee_module, "MyApp.Accounts", "Callee module should be MyApp.Accounts");
            assert_eq!(step.callee_function, "get_user", "Callee function should be get_user");
        }
    }

    #[rstest]
    fn test_reverse_trace_calls_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(&*populated_db, "[invalid", "get_user", None, "default", true, 10, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_reverse_trace_calls_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(
            &*populated_db,
            "MyApp.Accounts",
            "get_user",
            None,
            "nonexistent",
            false,
            10,
            100,
        );
        assert!(result.is_ok());
        let steps = result.unwrap();
        assert!(steps.is_empty(), "Nonexistent project should return no results");
    }

    #[rstest]
    fn test_reverse_trace_calls_depth_field_populated(populated_db: Box<dyn crate::backend::Database>) {
        let result = reverse_trace_calls(&*populated_db, "MyApp.Accounts", "get_user", None, "default", false, 10, 100)
            .unwrap();

        // All steps should have depth >= 1
        for step in &result {
            assert!(step.depth >= 1, "Depth should be >= 1");
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_reverse_trace_calls_recursive_reverse_traversal() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: Notifier.send_email/2 is called by Service.process_request/2 and Controller.create/2
        // Recursive trace will also find Controller.create as depth-2 caller (via Service.process_request)
        let result = reverse_trace_calls(&*db, "MyApp.Notifier", "send_email", None, "default", false, 10, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let steps = result.unwrap();

        // Should find at least 2 callers (recursive trace includes transitive callers)
        assert!(steps.len() >= 2, "Should find at least 2 callers of send_email");

        // Filter for depth-1 callers
        let depth_1_steps: Vec<_> = steps.iter().filter(|s| s.depth == 1).collect();
        assert_eq!(depth_1_steps.len(), 2, "Should find exactly 2 direct callers at depth 1");

        // All depth-1 steps should have Notifier.send_email as callee
        for step in &depth_1_steps {
            assert_eq!(step.callee_module, "MyApp.Notifier");
            assert_eq!(step.callee_function, "send_email");
            assert_eq!(step.callee_arity, 2);
        }

        // Verify depth-1 callers (order may vary)
        let callers: Vec<(&str, &str, i64)> = depth_1_steps
            .iter()
            .map(|s| (s.caller_module.as_str(), s.caller_function.as_str(), s.caller_arity))
            .collect();
        assert!(
            callers.contains(&("MyApp.Controller", "create", 2)),
            "Should be called by Controller.create/2"
        );
        assert!(
            callers.contains(&("MyApp.Service", "process_request", 2)),
            "Should be called by Service.process_request/2"
        );
    }

    #[test]
    fn test_reverse_trace_calls_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = reverse_trace_calls(
            &*db,
            "NonExistent",
            "nonexistent",
            None,
            "default",
            false,
            10,
            100,
        );

        assert!(result.is_ok(), "Query should succeed");
        let steps = result.unwrap();
        assert!(
            steps.is_empty(),
            "Non-existent module should return no results"
        );
    }

    #[test]
    fn test_reverse_trace_calls_with_depth_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace callers of list_users/0 with depth limit 1
        // Expected: only direct callers at depth 1
        let shallow = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "list_users",
            None,
            "default",
            false,
            1,
            100,
        )
        .expect("Shallow query should succeed");

        assert_eq!(shallow.len(), 1, "Depth 1 should find exactly 1 caller");
        assert_eq!(shallow[0].depth, 1, "Should be at depth 1");
        assert_eq!(shallow[0].caller_module, "MyApp.Controller");
        assert_eq!(shallow[0].caller_function, "index");
        assert_eq!(shallow[0].callee_module, "MyApp.Accounts");
        assert_eq!(shallow[0].callee_function, "list_users");

        // Trace callers of list_users/0 with depth limit 5
        // Expected: deeper call chains
        let deep = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "list_users",
            None,
            "default",
            false,
            5,
            100,
        )
        .expect("Deep query should succeed");

        // Should have more or equal results with deeper depth
        assert!(
            deep.len() >= shallow.len(),
            "Deeper depth should return >= results"
        );
    }

    #[test]
    fn test_reverse_trace_calls_depth_field_populated() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Trace callers of list_users/0 (which is called by index/2)
        let result = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "list_users",
            None,
            "default",
            false,
            10,
            100,
        )
        .expect("Query should succeed");

        assert!(!result.is_empty(), "Should find callers");

        // All results should have depth field populated and > 0
        for step in &result {
            assert!(
                step.depth > 0,
                "Every step should have depth > 0, found {}",
                step.depth
            );
        }
    }

    #[test]
    fn test_reverse_trace_calls_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = reverse_trace_calls(&*db, "[invalid", "index", None, "default", true, 10, 100);

        assert!(result.is_err(), "Should reject invalid regex pattern");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern") || msg.contains("regex"),
            "Error should mention regex validation"
        );
    }

    #[test]
    fn test_reverse_trace_calls_with_arity_filter() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with arity filter
        let result = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "list_users",
            Some(0),
            "default",
            false,
            10,
            100,
        );

        assert!(result.is_ok(), "Query with arity filter should succeed");
    }

    #[test]
    fn test_reverse_trace_calls_module_function_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: Notifier.send_email/2 calls Notifier.format_message/1
        // Reverse trace of format_message should find send_email as the only caller
        // But trace is recursive, so it will also find callers of send_email
        let result = reverse_trace_calls(&*db, "MyApp.Notifier", "format_message", None, "default", false, 10, 100)
            .expect("Query should succeed");

        assert!(result.len() >= 1, "Should find at least 1 caller of format_message");

        // Filter for depth-1 callers
        let depth_1_steps: Vec<_> = result.iter().filter(|s| s.depth == 1).collect();
        assert_eq!(depth_1_steps.len(), 1, "Should find exactly 1 direct caller at depth 1");

        // The direct caller should be MyApp.Notifier.send_email
        assert_eq!(
            depth_1_steps[0].caller_module,
            "MyApp.Notifier",
            "Caller module should be MyApp.Notifier"
        );
        assert_eq!(
            depth_1_steps[0].caller_function,
            "send_email",
            "Caller name should be send_email"
        );
        assert_eq!(depth_1_steps[0].caller_arity, 2, "Caller arity should be 2");

        // The callee should be MyApp.Notifier.format_message
        assert_eq!(
            depth_1_steps[0].callee_module,
            "MyApp.Notifier",
            "Callee module should be MyApp.Notifier"
        );
        assert_eq!(
            depth_1_steps[0].callee_function,
            "format_message",
            "Callee name should be format_message"
        );
        assert_eq!(depth_1_steps[0].callee_arity, 1, "Callee arity should be 1");
    }

    #[test]
    fn test_reverse_trace_calls_all_fields_present() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "list_users",
            None,
            "default",
            false,
            10,
            100,
        )
        .expect("Query should succeed");

        assert!(!result.is_empty(), "Should have results");

        // Verify all fields are present and valid for each step
        for (i, step) in result.iter().enumerate() {
            assert!(
                !step.caller_module.is_empty(),
                "Step {}: Caller module should not be empty",
                i
            );
            assert!(
                !step.caller_function.is_empty(),
                "Step {}: Caller function should not be empty",
                i
            );
            assert!(
                step.caller_arity >= 0,
                "Step {}: Caller arity should be >= 0",
                i
            );
            assert!(
                !step.callee_module.is_empty(),
                "Step {}: Callee module should not be empty",
                i
            );
            assert!(
                !step.callee_function.is_empty(),
                "Step {}: Callee function should not be empty",
                i
            );
            assert!(
                step.callee_arity >= 0,
                "Step {}: Callee arity should be >= 0",
                i
            );
            assert!(step.depth >= 1, "Step {}: Depth should be >= 1", i);
        }
    }

    #[test]
    fn test_reverse_trace_calls_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "all",
            None,
            "default",
            false,
            10,
            1,
        )
        .unwrap_or_default();

        let limit_10 = reverse_trace_calls(
            &*db,
            "MyApp.Accounts",
            "all",
            None,
            "default",
            false,
            10,
            10,
        )
        .unwrap_or_default();

        // Higher limit should return >= results
        assert!(
            limit_1.len() <= limit_10.len(),
            "Higher limit should return >= results"
        );
    }

    #[test]
    fn test_reverse_trace_calls_zero_depth_returns_empty() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // max_depth of 0 should return empty results
        let result = reverse_trace_calls(
            &*db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            0,
            100,
        )
        .unwrap_or_default();

        assert!(result.is_empty(), "Depth 0 should return no results");
    }

    #[test]
    fn test_reverse_trace_from_repo_query_deep_call_chain() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Repo.query/2 is a "leaf" function called by many paths in the complex fixture:
        // - Repo.get/2 -> Repo.query/2
        // - Repo.all/1 -> Repo.query/2
        // And those are called by:
        // - Accounts.get_user/1 -> Repo.get/2
        // - Accounts.list_users/0 -> Repo.all/1
        // And those are called by:
        // - Controller.index/2 -> Accounts.list_users/0
        // - Controller.show/2 -> Accounts.get_user/2 -> Accounts.get_user/1
        // - Controller.create/2 -> Service.process_request/2 -> Accounts.get_user/1
        // etc.

        let result = reverse_trace_calls(
            &*db,
            "MyApp.Repo",
            "query",
            Some(2), // arity 2
            "default",
            false,
            10,   // high depth to get all callers
            1000, // high limit
        )
        .expect("Query should succeed");

        eprintln!("=== Reverse trace from Repo.query/2 ===");
        eprintln!("Total steps found: {}", result.len());

        // Group by depth for visibility
        let mut by_depth: std::collections::HashMap<i64, Vec<&ReverseTraceStep>> =
            std::collections::HashMap::new();
        for step in &result {
            by_depth.entry(step.depth).or_default().push(step);
        }

        for depth in 1..=10 {
            if let Some(steps) = by_depth.get(&depth) {
                eprintln!("\nDepth {}:", depth);
                for step in steps {
                    eprintln!(
                        "  {}.{}/{} calls {}.{}/{}",
                        step.caller_module,
                        step.caller_function,
                        step.caller_arity,
                        step.callee_module,
                        step.callee_function,
                        step.callee_arity
                    );
                }
            }
        }

        // Verify we find direct callers at depth 1
        let depth_1: Vec<_> = result.iter().filter(|s| s.depth == 1).collect();
        assert!(
            depth_1.len() >= 2,
            "Should find at least 2 direct callers (Repo.get and Repo.all), found {}",
            depth_1.len()
        );

        // Verify Repo.get calls Repo.query
        let repo_get_calls = depth_1
            .iter()
            .find(|s| s.caller_module == "MyApp.Repo" && s.caller_function == "get");
        assert!(
            repo_get_calls.is_some(),
            "Should find Repo.get as a caller of Repo.query"
        );

        // Verify Repo.all calls Repo.query
        let repo_all_calls = depth_1
            .iter()
            .find(|s| s.caller_module == "MyApp.Repo" && s.caller_function == "all");
        assert!(
            repo_all_calls.is_some(),
            "Should find Repo.all as a caller of Repo.query"
        );

        // Verify we find callers at depth 2 (callers of Repo.get and Repo.all)
        let depth_2: Vec<_> = result.iter().filter(|s| s.depth == 2).collect();
        assert!(
            !depth_2.is_empty(),
            "Should find callers at depth 2 (e.g., Accounts.get_user calling Repo.get)"
        );

        // Verify we reach deeper into the call graph
        let max_depth = result.iter().map(|s| s.depth).max().unwrap_or(0);
        assert!(
            max_depth >= 3,
            "Should trace at least 3 levels deep, found max depth {}",
            max_depth
        );
    }
}
