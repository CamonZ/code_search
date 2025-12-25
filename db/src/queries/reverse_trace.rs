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
