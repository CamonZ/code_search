use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};

#[cfg(feature = "backend-cozo")]
use std::collections::HashMap;
#[cfg(feature = "backend-cozo")]
use crate::db::{extract_i64, extract_string, run_query};
#[cfg(feature = "backend-cozo")]
use crate::query_builders::OptionalConditionBuilder;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path query failed: {message}")]
    QueryFailed { message: String },
    #[error("Arity required: {message}")]
    ArityRequired { message: String },
}

/// A single step in a call path
#[derive(Debug, Clone, Serialize)]
pub struct PathStep {
    pub depth: i64,
    pub caller_module: String,
    pub caller_function: String,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
}

/// A complete path from source to target
#[derive(Debug, Clone, Serialize)]
pub struct CallPath {
    pub steps: Vec<PathStep>,
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
#[allow(clippy::too_many_arguments)]
pub fn find_paths(
    db: &dyn Database,
    from_module: &str,
    from_function: &str,
    from_arity: i64,
    to_module: &str,
    to_function: &str,
    to_arity: i64,
    _project: &str,
    max_depth: u32,
    _limit: u32,
) -> Result<Vec<CallPath>, Box<dyn Error>> {
    // Build the shortest path query using SurrealDB's shortest path operator
    // Uses parameter substitution for record ID construction
    // {..max_depth+shortest=target+inclusive} finds shortest path from source to target
    // +inclusive includes the origin in the result
    let query = format!(
        r#"SELECT @.{{..{}+shortest=functions:[$target_module, $target_fn, $target_arity]+inclusive}}->calls->functions AS path FROM functions:[$source_module, $source_fn, $source_arity];"#,
        max_depth
    );

    let params = QueryParams::new()
        .with_str("source_module", from_module)
        .with_str("source_fn", from_function)
        .with_int("source_arity", from_arity)
        .with_str("target_module", to_module)
        .with_str("target_fn", to_function)
        .with_int("target_arity", to_arity);

    let result = db.execute_query(&query, params)
        .map_err(|e| PathError::QueryFailed {
            message: e.to_string(),
        })?;

    // Parse the path result
    let mut all_paths: Vec<CallPath> = Vec::new();

    for row in result.rows().iter() {
        if let Some(path) = row.get(0).and_then(|v| v.as_array()) {
            // Convert path array into CallPath
            let steps = convert_path_to_steps(db, &path)?;
            if !steps.is_empty() {
                all_paths.push(CallPath { steps });
            }
        }
    }

    Ok(all_paths)
}

/// Convert a SurrealDB path array to CallPath steps
#[cfg(feature = "backend-surrealdb")]
fn convert_path_to_steps(db: &dyn Database, path: &[&dyn crate::backend::Value]) -> Result<Vec<PathStep>, Box<dyn Error>> {
    let mut steps = Vec::new();

    // Path contains nodes, we need to convert consecutive pairs into steps
    // Each step represents a call from one function to another
    for window in path.windows(2) {
        if let (Some(caller), Some(callee)) = (
            extract_function_data(window[0]),
            extract_function_data(window[1]),
        ) {
            // Look up the call edge to get the line number and file
            let (line, file) = lookup_call_edge(db, &caller, &callee);

            let depth = (steps.len() + 1) as i64;
            steps.push(PathStep {
                depth,
                caller_module: caller.0,
                caller_function: caller.1,
                callee_module: callee.0,
                callee_function: callee.1,
                callee_arity: callee.2,
                file,
                line,
            });
        }
    }

    Ok(steps)
}

/// Look up the call edge between two functions to get line number and file
#[cfg(feature = "backend-surrealdb")]
fn lookup_call_edge(
    db: &dyn Database,
    caller: &(String, String, i64),
    callee: &(String, String, i64),
) -> (i64, String) {
    let edge_query = r#"
        SELECT line, file
        FROM calls
        WHERE in = functions:[$caller_module, $caller_name, $caller_arity]
          AND out = functions:[$callee_module, $callee_name, $callee_arity]
        LIMIT 1;
    "#;

    let edge_params = QueryParams::new()
        .with_str("caller_module", &caller.0)
        .with_str("caller_name", &caller.1)
        .with_int("caller_arity", caller.2)
        .with_str("callee_module", &callee.0)
        .with_str("callee_name", &callee.1)
        .with_int("callee_arity", callee.2);

    match db.execute_query(edge_query, edge_params) {
        Ok(edge_result) => {
            let headers = edge_result.headers();
            if let Some(edge_row) = edge_result.rows().first() {
                // Use header indices because SurrealDB returns columns in alphabetical order,
                // not in SELECT clause order
                let line_idx = headers.iter().position(|h| h == "line");
                let file_idx = headers.iter().position(|h| h == "file");

                let line = line_idx
                    .and_then(|idx| edge_row.get(idx))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let file = file_idx
                    .and_then(|idx| edge_row.get(idx))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (line, file)
            } else {
                (0, String::new())
            }
        }
        Err(_) => (0, String::new()),
    }
}

/// Extract function data from a SurrealDB Thing value
/// Returns (module, name, arity)
#[cfg(feature = "backend-surrealdb")]
fn extract_function_data(value: &dyn crate::backend::Value) -> Option<(String, String, i64)> {
    let id = value.as_thing_id()?;
    let parts = id.as_array()?;

    let module = parts.get(0)?.as_str()?.to_string();
    let name = parts.get(1)?.as_str()?.to_string();
    let arity = parts.get(2)?.as_i64()?;

    Some((module, name, arity))
}


// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
#[allow(clippy::too_many_arguments)]
pub fn find_paths(
    db: &dyn Database,
    from_module: &str,
    from_function: &str,
    from_arity: i64,
    to_module: &str,
    to_function: &str,
    to_arity: i64,
    project: &str,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<CallPath>, Box<dyn Error>> {
    // Build conditions using the ConditionBuilder utilities
    // Arity is now required, so we always include the condition
    let from_arity_cond = OptionalConditionBuilder::new("caller_arity", "from_arity")
        .when_none("true")
        .build(true);

    let to_arity_cond = OptionalConditionBuilder::new("callee_arity", "to_arity")
        .when_none("true")
        .build(true);

    // Simpler approach: trace forward from source to find all reachable calls,
    // then filter to paths that end at the target.
    // Returns edges on valid paths (may include multiple paths if they exist).
    // Joins with function_locations to get caller arity for filtering.
    let script = format!(
        r#"
        # Base case: direct calls from the source function
        # Join with function_locations to get caller arity
        # Uses starts_with to handle both "func" and "func/2" formats in caller_function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity}},
            starts_with(caller_function, caller_name),
            caller_module == $from_module,
            starts_with(caller_function, $from_function),
            {from_arity_cond},
            project == $project,
            depth = 1

        # Recursive case: continue from callees we've found
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            trace[prev_depth, _, _, prev_callee_module, prev_callee_function, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            caller_module == prev_callee_module,
            starts_with(caller_function, prev_callee_function),
            prev_depth < {max_depth},
            depth = prev_depth + 1,
            project == $project

        # Find the depth at which we reach the target
        target_depth[d] :=
            trace[d, _, _, callee_module, callee_function, callee_arity, _, _],
            callee_module == $to_module,
            starts_with(callee_function, $to_function),
            {to_arity_cond}

        # Only return edges at depths <= minimum target depth (edges on valid paths)
        ?[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line],
            target_depth[min_d],
            depth <= min_d

        :order depth, caller_module, caller_function, callee_module, callee_function
        :limit {limit}
        "#,
    );

    let params = QueryParams::new()
        .with_str("from_module", from_module)
        .with_str("from_function", from_function)
        .with_int("from_arity", from_arity)
        .with_str("to_module", to_module)
        .with_str("to_function", to_function)
        .with_int("to_arity", to_arity)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| PathError::QueryFailed {
        message: e.to_string(),
    })?;

    // Parse all edges from the query result
    let mut edges: Vec<PathStep> = Vec::new();

    for row in result.rows() {
        if row.len() >= 8 {
            let depth = extract_i64(row.get(0).unwrap(), 0);
            let Some(caller_module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(caller_function) = extract_string(row.get(2).unwrap()) else { continue };
            let Some(callee_module) = extract_string(row.get(3).unwrap()) else { continue };
            let Some(callee_function) = extract_string(row.get(4).unwrap()) else { continue };
            let callee_arity = extract_i64(row.get(5).unwrap(), 0);
            let Some(file) = extract_string(row.get(6).unwrap()) else { continue };
            let line = extract_i64(row.get(7).unwrap(), 0);

            edges.push(PathStep {
                depth,
                caller_module,
                caller_function,
                callee_module,
                callee_function,
                callee_arity,
                file,
                line,
            });
        }
    }

    if edges.is_empty() {
        return Ok(vec![]);
    }

    // Build adjacency list: (module, function) -> list of edges from that node
    // Key is (caller_module, caller_function), value is list of edges
    let mut adj: HashMap<(String, String), Vec<&PathStep>> = HashMap::new();
    for edge in &edges {
        adj.entry((edge.caller_module.clone(), edge.caller_function.clone()))
            .or_default()
            .push(edge);
    }

    // Find all paths using DFS from source to target
    let mut all_paths: Vec<CallPath> = Vec::new();
    let mut current_path: Vec<PathStep> = Vec::new();

    // Find starting edges (depth 1, from the source function)
    let starting_edges: Vec<&PathStep> = edges.iter().filter(|e| e.depth == 1).collect();

    for start_edge in starting_edges {
        current_path.clear();
        dfs_find_paths(
            start_edge,
            to_module,
            to_function,
            to_arity,
            &adj,
            &mut current_path,
            &mut all_paths,
            limit as usize,
        );
    }

    Ok(all_paths)
}

/// DFS to find all paths from current edge to target
#[cfg(feature = "backend-cozo")]
fn dfs_find_paths(
    current_edge: &PathStep,
    to_module: &str,
    to_function: &str,
    to_arity: i64,
    adj: &HashMap<(String, String), Vec<&PathStep>>,
    current_path: &mut Vec<PathStep>,
    all_paths: &mut Vec<CallPath>,
    limit: usize,
) {
    // Add current edge to path
    current_path.push(current_edge.clone());

    // Check if we reached the target
    let at_target = current_edge.callee_module == to_module
        && current_edge.callee_function == to_function
        && current_edge.callee_arity == to_arity;

    if at_target {
        // Found a complete path
        all_paths.push(CallPath {
            steps: current_path.clone(),
        });
    } else if all_paths.len() < limit {
        // Continue searching from the callee
        // Find edges where caller matches our callee
        // Note: caller_function has arity suffix, callee_function doesn't
        // So we need to find edges where caller starts with our callee_function
        for (key, next_edges) in adj.iter() {
            if key.0 == current_edge.callee_module && key.1.starts_with(&current_edge.callee_function) {
                for next_edge in next_edges {
                    // Avoid cycles - check if we've already visited this exact edge
                    let already_visited = current_path.iter().any(|e| {
                        e.caller_module == next_edge.caller_module
                            && e.caller_function == next_edge.caller_function
                            && e.callee_module == next_edge.callee_module
                            && e.callee_function == next_edge.callee_function
                    });

                    if !already_visited && all_paths.len() < limit {
                        dfs_find_paths(
                            next_edge,
                            to_module,
                            to_function,
                            to_arity,
                            adj,
                            current_path,
                            all_paths,
                            limit,
                        );
                    }
                }
            }
        }
    }

    // Backtrack
    current_path.pop();
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
    fn test_find_paths_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "list_users",
            0,
            "default",
            10,
            100,
        );
        assert!(result.is_ok());
        let paths = result.unwrap();
        // Should find exactly one path: Controller.index/2 -> Accounts.list_users/0
        assert_eq!(paths.len(), 1, "Should find exactly 1 path from Controller.index/2 to Accounts.list_users/0");

        // Verify the path structure
        let path = &paths[0];
        assert_eq!(path.steps.len(), 1, "Should be a direct call (1 step)");

        let step = &path.steps[0];
        assert_eq!(step.caller_module, "MyApp.Controller", "Caller module mismatch");
        // caller_function may have arity suffix from fixture (e.g., "index/2")
        assert!(step.caller_function.starts_with("index"), "Caller function should be index");
        assert_eq!(step.callee_module, "MyApp.Accounts", "Callee module mismatch");
        assert_eq!(step.callee_function, "list_users", "Callee function mismatch");
        assert_eq!(step.callee_arity, 0, "Callee arity should be 0");
    }

    #[rstest]
    fn test_find_paths_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_paths(
            &*populated_db,
            "NonExistent",
            "nonexistent",
            1,
            "MyApp.Accounts",
            "validate_email",
            1,
            "default",
            10,
            100,
        );
        assert!(result.is_ok());
        let paths = result.unwrap();
        // No paths from non-existent source
        assert_eq!(paths.len(), 0, "No paths should be found from non-existent source");
    }

    #[rstest]
    fn test_find_paths_unreachable_target(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_paths(
            &*populated_db,
            "MyApp.Accounts",
            "validate_email",
            1,
            "MyApp.Controller",
            "index",
            2,
            "default",
            10,
            100,
        );
        assert!(result.is_ok());
        let paths = result.unwrap();
        // No paths if target is not reachable from source (Accounts does not call Controller)
        assert_eq!(paths.len(), 0, "No paths should be found to unreachable target");
    }

    #[rstest]
    fn test_find_paths_with_arity_filters(populated_db: Box<dyn crate::backend::Database>) {
        // Test with correct arity - should find no path (Controller.index is /2, not /1)
        let result = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            1,  // Wrong arity - Controller.index is /2
            "MyApp.Accounts",
            "validate_email",
            1,
            "default",
            10,
            100,
        );
        assert!(result.is_ok());
        let paths = result.unwrap();
        // Should return empty because Controller.index/1 doesn't exist
        assert_eq!(paths.len(), 0, "Wrong arity should return no paths");

        // Test with correct arity - should find path
        let result_correct = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,  // Correct arity - Controller.index is /2
            "MyApp.Accounts",
            "list_users",
            0,
            "default",
            10,
            100,
        );
        assert!(result_correct.is_ok());
        let paths_correct = result_correct.unwrap();
        // Should find paths with correct arity
        assert!(!paths_correct.is_empty(), "Correct arity should find paths");
        for path in &paths_correct {
            assert!(!path.steps.is_empty(), "Path should have at least one step");
        }
    }

    #[rstest]
    fn test_find_paths_respects_max_depth(populated_db: Box<dyn crate::backend::Database>) {
        let shallow = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "get_user",
            1,
            "default",
            2,
            100,
        )
        .unwrap();

        let deep = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "get_user",
            1,
            "default",
            10,
            100,
        )
        .unwrap();

        // Deeper search may find more paths
        // Shallow should have same or fewer
        assert!(shallow.len() <= deep.len(), "Shallow depth should find same or fewer paths than deep depth");
    }

    #[rstest]
    fn test_find_paths_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_1 = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "get_user",
            1,
            "default",
            10,
            1,
        )
        .unwrap();

        let limit_10 = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "get_user",
            1,
            "default",
            10,
            10,
        )
        .unwrap();

        // Smaller limit should return fewer or equal paths
        assert!(limit_1.len() <= limit_10.len(), "Smaller limit should return fewer paths");
        assert!(limit_1.len() <= 1, "Limit of 1 should return at most 1 path");
    }

    #[rstest]
    fn test_find_paths_path_steps_valid(populated_db: Box<dyn crate::backend::Database>) {
        // Controller.show/2 -> Accounts.get_user/1 -> Repo.get/2 (2 hop path)
        let result = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "show",
            2,
            "MyApp.Repo",
            "get",
            2,
            "default",
            10,
            100,
        )
        .unwrap();

        assert!(!result.is_empty(), "Should find at least one path");
        for path in &result {
            assert!(!path.steps.is_empty(), "Each path should have at least one step");
            // Verify path continuity - each step's callee should match next step's caller
            for i in 0..path.steps.len() - 1 {
                let current = &path.steps[i];
                let next = &path.steps[i + 1];
                assert_eq!(current.callee_module, next.caller_module, "Step {} callee should match next step caller module", i);
                // Caller function may have arity suffix (e.g., "get_user/2"), so check that it starts with the callee function name
                assert!(next.caller_function.starts_with(&current.callee_function),
                    "Step {} callee {} should match next step caller function {}", i, current.callee_function, next.caller_function);
            }
            // Each step should have valid data
            for (idx, step) in path.steps.iter().enumerate() {
                assert!(!step.caller_module.is_empty(), "Step {} caller module should not be empty", idx);
                assert!(!step.caller_function.is_empty(), "Step {} caller function should not be empty", idx);
                assert!(!step.callee_module.is_empty(), "Step {} callee module should not be empty", idx);
                assert!(!step.callee_function.is_empty(), "Step {} callee function should not be empty", idx);
                assert!(step.callee_arity >= 0, "Step {} callee arity should be non-negative", idx);
            }
        }
    }

    #[rstest]
    fn test_find_paths_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_paths(
            &*populated_db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "get_user",
            1,
            "nonexistent",
            10,
            100,
        );
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 0, "Nonexistent project should return no paths");
    }
}

// ==================== SurrealDB Tests ====================
#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_paths_shortest_path() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test shortest path: Controller.create/2 -> Notifier.send_email/2
        // Two paths exist:
        // - Short path (1 hop): Controller.create/2 -> Notifier.send_email/2
        // - Long path (2 hops): Controller.create/2 -> Service.process_request/2 -> Notifier.send_email/2
        // The algorithm should return the 1-hop path
        let result = find_paths(
            &*db,
            "MyApp.Controller",
            "create",
            2,
            "MyApp.Notifier",
            "send_email",
            2,
            "default",
            10,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should find exactly 1 path");
        assert_eq!(paths[0].steps.len(), 1, "Shortest path should have exactly 1 step (direct call)");

        let step = &paths[0].steps[0];
        assert_eq!(step.caller_module, "MyApp.Controller", "Caller should be Controller");
        assert_eq!(step.caller_function, "create", "Caller function should be create");
        assert_eq!(step.callee_module, "MyApp.Notifier", "Callee should be Notifier");
        assert_eq!(step.callee_function, "send_email", "Callee function should be send_email");
        assert_eq!(step.callee_arity, 2, "Callee arity should be 2");
        assert_eq!(step.depth, 1, "Step depth should be 1");
    }

    #[test]
    fn test_find_paths_with_max_depth() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Path from Controller.show/2 to Repo.query/2 requires 4 hops:
        // Controller.show/2 -> Accounts.get_user/2 -> Accounts.get_user/1 -> Repo.get/2 -> Repo.query/2

        // With max_depth=2, should find 0 paths (target is 4 hops away)
        let shallow = find_paths(
            &*db,
            "MyApp.Controller",
            "show",
            2,
            "MyApp.Repo",
            "query",
            2,
            "default",
            2,
            100,
        );

        assert!(shallow.is_ok(), "Shallow query should succeed: {:?}", shallow.err());
        let shallow_paths = shallow.unwrap();
        assert_eq!(shallow_paths.len(), 0, "max_depth=2 should find 0 paths (target is 4 hops away)");

        // With max_depth=5, should find exactly 1 path
        let deep = find_paths(
            &*db,
            "MyApp.Controller",
            "show",
            2,
            "MyApp.Repo",
            "query",
            2,
            "default",
            5,
            100,
        );

        assert!(deep.is_ok(), "Deep query should succeed: {:?}", deep.err());
        let deep_paths = deep.unwrap();
        assert_eq!(deep_paths.len(), 1, "max_depth=5 should find exactly 1 path");
        assert_eq!(deep_paths[0].steps.len(), 4, "Path should have exactly 4 steps");

        // Validate path continuity: each step's callee should match the next step's caller
        let steps = &deep_paths[0].steps;
        assert_eq!(steps[0].caller_function, "show", "First step should start from show");
        assert_eq!(steps[0].callee_function, "get_user", "First step should call get_user");
        for i in 0..steps.len() - 1 {
            assert_eq!(
                steps[i].callee_module, steps[i + 1].caller_module,
                "Step {} callee module should match step {} caller module", i, i + 1
            );
            assert_eq!(
                steps[i].callee_function, steps[i + 1].caller_function,
                "Step {} callee function should match step {} caller function", i, i + 1
            );
        }
        assert_eq!(steps[3].callee_function, "query", "Last step should end at query");
    }

    #[test]
    fn test_find_paths_no_path_exists() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Try to find path from Accounts to Controller (impossible - Controller calls Accounts)
        let result = find_paths(
            &*db,
            "MyApp.Accounts",
            "list_users",
            0,
            "MyApp.Controller",
            "index",
            2,
            "default",
            10,
            100,
        );

        assert!(result.is_ok(), "Query should handle non-existent paths gracefully");
        let paths = result.unwrap();
        assert!(paths.is_empty(), "No path should exist from Accounts.list_users to Controller.index");
    }

    #[test]
    fn test_find_paths_nonexistent_source() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test that querying from a non-existent function returns 0 paths without error
        let result = find_paths(
            &*db,
            "NonExistent",
            "nonexistent",
            1,
            "MyApp.Accounts",
            "list_users",
            0,
            "default",
            10,
            100,
        );

        assert!(result.is_ok(), "Query should succeed even for non-existent source: {:?}", result.err());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 0, "Non-existent source should return exactly 0 paths");
    }

    #[test]
    fn test_find_paths_nonexistent_target() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test that querying to a non-existent target returns 0 paths without error
        let result = find_paths(
            &*db,
            "MyApp.Controller",
            "index",
            2,
            "NonExistent",
            "nonexistent",
            1,
            "default",
            10,
            100,
        );

        assert!(result.is_ok(), "Query should succeed even for non-existent target: {:?}", result.err());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 0, "Non-existent target should return exactly 0 paths");
    }

    #[test]
    fn test_find_paths_path_steps_validity() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test path: Controller.index/2 -> Accounts.list_users/0 -> Repo.all/1
        // This is a 2-hop path that validates all PathStep fields
        let result = find_paths(
            &*db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Repo",
            "all",
            1,
            "default",
            5,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should find exactly 1 path");
        assert_eq!(paths[0].steps.len(), 2, "Path should have exactly 2 steps");

        // Validate Step 1: Controller.index/2 -> Accounts.list_users/0
        let step1 = &paths[0].steps[0];
        assert_eq!(step1.depth, 1, "Step 1 depth should be 1");
        assert_eq!(step1.caller_module, "MyApp.Controller", "Step 1 caller module");
        assert_eq!(step1.caller_function, "index", "Step 1 caller function");
        assert_eq!(step1.callee_module, "MyApp.Accounts", "Step 1 callee module");
        assert_eq!(step1.callee_function, "list_users", "Step 1 callee function");
        assert_eq!(step1.callee_arity, 0, "Step 1 callee arity");

        // Validate Step 2: Accounts.list_users/0 -> Repo.all/1
        let step2 = &paths[0].steps[1];
        assert_eq!(step2.depth, 2, "Step 2 depth should be 2");
        assert_eq!(step2.caller_module, "MyApp.Accounts", "Step 2 caller module");
        assert_eq!(step2.caller_function, "list_users", "Step 2 caller function");
        assert_eq!(step2.callee_module, "MyApp.Repo", "Step 2 callee module");
        assert_eq!(step2.callee_function, "all", "Step 2 callee function");
        assert_eq!(step2.callee_arity, 1, "Step 2 callee arity");

        // Validate path continuity: step1 callee == step2 caller
        assert_eq!(step1.callee_module, step2.caller_module, "Step continuity: callee module matches next caller module");
        assert_eq!(step1.callee_function, step2.caller_function, "Step continuity: callee function matches next caller function");
    }

    #[test]
    fn test_find_paths_simple_graph() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Controller.index/2 -> Accounts.list_users/0 (direct call in complex fixture)
        let result = find_paths(
            &*db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "list_users",
            0,
            "default",
            10,
            100,
        );

        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should find exactly 1 path in simple graph");

        let path = &paths[0];
        assert_eq!(path.steps.len(), 1, "Direct call should have 1 step");
        assert_eq!(path.steps[0].caller_module, "MyApp.Controller");
        assert_eq!(path.steps[0].caller_function, "index");
        assert_eq!(path.steps[0].callee_module, "MyApp.Accounts");
        assert_eq!(path.steps[0].callee_function, "list_users");
        assert_eq!(path.steps[0].depth, 1);
    }

    #[test]
    fn test_find_paths_returns_line_numbers() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test path: Controller.index/2 -> Accounts.list_users/0
        // The fixture has this call at line 7
        let result = find_paths(
            &*db,
            "MyApp.Controller",
            "index",
            2,
            "MyApp.Accounts",
            "list_users",
            0,
            "default",
            10,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should find exactly 1 path");

        let step = &paths[0].steps[0];
        assert_eq!(step.line, 7, "Call line should be 7 (from fixture)");
        assert_eq!(step.file, "lib/my_app/controller.ex", "File should match fixture");
    }

    #[test]
    fn test_lookup_call_edge_returns_correct_data() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test direct edge lookup
        let caller = ("MyApp.Controller".to_string(), "index".to_string(), 2i64);
        let callee = ("MyApp.Accounts".to_string(), "list_users".to_string(), 0i64);

        let (line, file) = lookup_call_edge(&*db, &caller, &callee);

        assert_eq!(line, 7, "Call line should be 7 (from fixture)");
        assert_eq!(file, "lib/my_app/controller.ex", "File should match fixture");
    }

    #[test]
    fn test_debug_edge_query() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Query with hardcoded record IDs
        let hardcoded_query = r#"
            SELECT line, file FROM calls
            WHERE in = functions:["MyApp.Controller", "index", 2]
              AND out = functions:["MyApp.Accounts", "list_users", 0]
        "#;
        let hardcoded_result = db.execute_query(hardcoded_query, QueryParams::new()).unwrap();

        // Show headers to understand column ordering
        let headers = hardcoded_result.headers();
        eprintln!("\nHeaders: {:?}", headers);
        eprintln!("(Note: SELECT was 'line, file' but headers may be alphabetically sorted)");

        eprintln!("\nHardcoded query result: {} rows", hardcoded_result.rows().len());
        for (i, row) in hardcoded_result.rows().iter().enumerate() {
            // Show what's at each index with type info
            for col_idx in 0..row.len() {
                let val = row.get(col_idx);
                let header = headers.get(col_idx).map(|s| s.as_str()).unwrap_or("?");
                let type_info = match val {
                    Some(v) if v.as_i64().is_some() => format!("i64: {}", v.as_i64().unwrap()),
                    Some(v) if v.as_str().is_some() => format!("str: {}", v.as_str().unwrap()),
                    Some(_) => "other".to_string(),
                    None => "None".to_string(),
                };
                eprintln!("  Row {} col {} ({}): {}", i, col_idx, header, type_info);
            }
        }

        // The test should pass if hardcoded works
        assert!(hardcoded_result.rows().len() > 0, "Hardcoded query should find the edge");

        // Verify we can access values using header names to find indices
        let row = hardcoded_result.rows().first().unwrap();
        let line_idx = headers.iter().position(|h| h == "line");
        let file_idx = headers.iter().position(|h| h == "file");

        eprintln!("\nColumn indices: line={:?}, file={:?}", line_idx, file_idx);

        if let Some(idx) = line_idx {
            let line = row.get(idx).and_then(|v| v.as_i64());
            eprintln!("line value via header index: {:?}", line);
            assert!(line.is_some(), "Should be able to access line by header index");
            assert_eq!(line.unwrap(), 7, "line should be 7 from fixture");
        } else {
            panic!("'line' header not found");
        }

        if let Some(idx) = file_idx {
            let file = row.get(idx).and_then(|v| v.as_str());
            eprintln!("file value via header index: {:?}", file);
            assert!(file.is_some(), "Should be able to access file by header index");
            assert_eq!(file.unwrap(), "lib/my_app/controller.ex", "file should match fixture");
        } else {
            panic!("'file' header not found");
        }
    }
}
