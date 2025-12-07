use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::PathCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum PathError {
    #[error("Path query failed: {message}")]
    QueryFailed { message: String },
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

/// Result of the path command execution
#[derive(Debug, Default, Serialize)]
pub struct PathResult {
    pub from_module: String,
    pub from_function: String,
    pub to_module: String,
    pub to_function: String,
    pub max_depth: u32,
    pub paths: Vec<CallPath>,
}

impl Execute for PathCmd {
    type Output = PathResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = PathResult {
            from_module: self.from_module.clone(),
            from_function: self.from_function.clone(),
            to_module: self.to_module.clone(),
            to_function: self.to_function.clone(),
            max_depth: self.depth,
            ..Default::default()
        };

        result.paths = find_paths(
            &db,
            &self.from_module,
            &self.from_function,
            self.from_arity,
            &self.to_module,
            &self.to_function,
            self.to_arity,
            &self.project,
            self.depth,
            self.limit,
        )?;

        Ok(result)
    }
}

#[allow(clippy::too_many_arguments)]
fn find_paths(
    db: &cozo::DbInstance,
    from_module: &str,
    from_function: &str,
    _from_arity: Option<i64>,
    to_module: &str,
    to_function: &str,
    to_arity: Option<i64>,
    project: &str,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<CallPath>, Box<dyn Error>> {
    let project_cond = ", project == $project";

    let to_arity_cond = if to_arity.is_some() {
        ", callee_arity == $to_arity"
    } else {
        ""
    };

    // Simpler approach: trace forward from source to find all reachable calls,
    // then filter to paths that end at the target.
    // Returns edges on valid paths (may include multiple paths if they exist).
    let script = format!(
        r#"
        # Base case: direct calls from the source function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            caller_module == $from_module,
            caller_function == $from_function
            {project_cond},
            depth = 1

        # Recursive case: continue from callees we've found
        # Note: caller_function has arity suffix (e.g., "foo/2") but callee_function doesn't (e.g., "foo")
        # So we use starts_with to match caller_function starting with prev_callee_function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            trace[prev_depth, _, _, prev_callee_module, prev_callee_function, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            caller_module == prev_callee_module,
            starts_with(caller_function, prev_callee_function),
            prev_depth < {max_depth},
            depth = prev_depth + 1
            {project_cond}

        # Find the depth at which we reach the target
        target_depth[d] :=
            trace[d, _, _, callee_module, callee_function, callee_arity, _, _],
            callee_module == $to_module,
            callee_function == $to_function
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

    let mut params = Params::new();
    params.insert("from_module".to_string(), DataValue::Str(from_module.into()));
    params.insert("from_function".to_string(), DataValue::Str(from_function.into()));
    params.insert("to_module".to_string(), DataValue::Str(to_module.into()));
    params.insert("to_function".to_string(), DataValue::Str(to_function.into()));
    if let Some(a) = to_arity {
        params.insert("to_arity".to_string(), DataValue::from(a));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(&db, &script, params).map_err(|e| PathError::QueryFailed {
        message: e.to_string(),
    })?;

    // Parse all edges from the query result
    let mut edges: Vec<PathStep> = Vec::new();

    for row in rows.rows {
        if row.len() >= 8 {
            let depth = extract_i64(&row[0], 0);
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let Some(callee_module) = extract_string(&row[3]) else { continue };
            let Some(callee_function) = extract_string(&row[4]) else { continue };
            let callee_arity = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let line = extract_i64(&row[7], 0);

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
fn dfs_find_paths(
    current_edge: &PathStep,
    to_module: &str,
    to_function: &str,
    to_arity: Option<i64>,
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
        && to_arity.map_or(true, |a| current_edge.callee_arity == a);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::ImportCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_call_graph_json() -> &'static str {
        r#"{
            "structs": {},
            "function_locations": {
                "MyApp.Controller": {
                    "index/2": {"arity": 2, "name": "index", "file": "lib/controller.ex", "column": 3, "kind": "def", "start_line": 5, "end_line": 10}
                },
                "MyApp.Service": {
                    "fetch/1": {"arity": 1, "name": "fetch", "file": "lib/service.ex", "column": 3, "kind": "def", "start_line": 10, "end_line": 20}
                },
                "MyApp.Repo": {
                    "get/2": {"arity": 2, "name": "get", "file": "lib/repo.ex", "column": 3, "kind": "def", "start_line": 15, "end_line": 25}
                }
            },
            "calls": [
                {
                    "caller": {"module": "MyApp.Controller", "function": "index", "file": "lib/controller.ex", "line": 7, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 1, "function": "fetch", "module": "MyApp.Service"}
                },
                {
                    "caller": {"module": "MyApp.Service", "function": "fetch", "file": "lib/service.ex", "line": 15, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}
                },
                {
                    "caller": {"module": "MyApp.Repo", "function": "get", "file": "lib/repo.ex", "line": 20, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 1, "function": "query", "module": "Ecto.Query"}
                }
            ],
            "type_signatures": {}
        }"#
    }

    fn create_temp_json_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp file");
        file
    }

    #[fixture]
    fn populated_db() -> NamedTempFile {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let json_file = create_temp_json_file(sample_call_graph_json());

        let import_cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        import_cmd
            .execute(db_file.path())
            .expect("Import should succeed");

        db_file
    }

    #[rstest]
    fn test_path_direct_call(populated_db: NamedTempFile) {
        let cmd = PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "MyApp.Service".to_string(),
            to_function: "fetch".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        };
        let result = cmd.execute(populated_db.path()).expect("Path should succeed");
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].steps.len(), 1);
        assert_eq!(result.paths[0].steps[0].caller_module, "MyApp.Controller");
        assert_eq!(result.paths[0].steps[0].callee_module, "MyApp.Service");
    }

    #[rstest]
    fn test_path_two_hops(populated_db: NamedTempFile) {
        let cmd = PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        };
        let result = cmd.execute(populated_db.path()).expect("Path should succeed");
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].steps.len(), 2);
    }

    #[rstest]
    fn test_path_three_hops(populated_db: NamedTempFile) {
        let cmd = PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "Ecto.Query".to_string(),
            to_function: "query".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        };
        let result = cmd.execute(populated_db.path()).expect("Path should succeed");
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].steps.len(), 3);
    }

    #[rstest]
    fn test_path_no_path_exists(populated_db: NamedTempFile) {
        let cmd = PathCmd {
            from_module: "MyApp.Repo".to_string(),
            from_function: "get".to_string(),
            from_arity: None,
            to_module: "MyApp.Controller".to_string(),
            to_function: "index".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        };
        let result = cmd.execute(populated_db.path()).expect("Path should succeed");
        assert!(result.paths.is_empty());
    }

    #[rstest]
    fn test_path_depth_limit(populated_db: NamedTempFile) {
        let cmd = PathCmd {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            from_arity: None,
            to_module: "Ecto.Query".to_string(),
            to_function: "query".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 2, // Not enough to reach Ecto.Query (needs 3)
            limit: 10,
        };
        let result = cmd.execute(populated_db.path()).expect("Path should succeed");
        assert!(result.paths.is_empty());
    }

    #[rstest]
    fn test_path_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = PathCmd {
            from_module: "MyApp".to_string(),
            from_function: "foo".to_string(),
            from_arity: None,
            to_module: "MyApp".to_string(),
            to_function: "bar".to_string(),
            to_arity: None,
            project: "test_project".to_string(),
            depth: 10,
            limit: 10,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
