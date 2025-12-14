use crate::db::DatabaseBackend;
use std::collections::HashMap;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::QueryBuilder;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("Path query failed: {message}")]
    QueryFailed { message: String },
}

/// Query builder for finding call paths between two functions
#[derive(Debug)]
pub struct PathQueryBuilder {
    pub from_module: String,
    pub from_function: String,
    pub from_arity: Option<i64>,
    pub to_module: String,
    pub to_function: String,
    pub to_arity: Option<i64>,
    pub project: String,
    pub max_depth: u32,
    pub limit: u32,
}

impl QueryBuilder for PathQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("from_module".to_string(), DataValue::Str(self.from_module.clone().into()));
        params.insert("from_function".to_string(), DataValue::Str(self.from_function.clone().into()));
        params.insert("to_module".to_string(), DataValue::Str(self.to_module.clone().into()));
        params.insert("to_function".to_string(), DataValue::Str(self.to_function.clone().into()));
        if let Some(a) = self.to_arity {
            params.insert("to_arity".to_string(), DataValue::from(a));
        }
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params
    }
}

impl PathQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let to_arity_cond = if self.to_arity.is_some() {
            ", callee_arity == $to_arity"
        } else {
            ""
        };

        Ok(format!(
            r#"
        # Base case: direct calls from the source function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            caller_module == $from_module,
            caller_function == $from_function,
            project == $project,
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
            depth = prev_depth + 1,
            project == $project

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
            max_depth = self.max_depth,
            limit = self.limit,
            to_arity_cond = to_arity_cond
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        Ok(format!(
            r#"MATCH path = (source:Function)-[:CALLS*1..{max_depth}]->(target:Function)
WHERE source.module = $from_module
  AND source.name = $from_function
  AND source.project = $project
  AND target.module = $to_module
  AND target.name = $to_function
  AND ($to_arity IS NULL OR target.arity = $to_arity)
WITH path, length(path) as depth,
     nodes(path) as funcs,
     relationships(path) as calls
UNWIND range(0, size(calls)-1) as idx
RETURN depth,
       funcs[idx].module as caller_module,
       funcs[idx].name as caller_function,
       funcs[idx+1].module as callee_module,
       funcs[idx+1].name as callee_function,
       funcs[idx+1].arity as callee_arity,
       calls[idx].file as file,
       calls[idx].line as line
ORDER BY depth, caller_module, caller_function
LIMIT {limit}"#,
            max_depth = self.max_depth,
            limit = self.limit
        ))
    }
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

#[allow(clippy::too_many_arguments)]
pub fn find_paths(
    db: &dyn DatabaseBackend,
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
    use crate::queries::builder::CompiledQuery;

    let builder = PathQueryBuilder {
        from_module: from_module.to_string(),
        from_function: from_function.to_string(),
        from_arity: None,
        to_module: to_module.to_string(),
        to_function: to_function.to_string(),
        to_arity,
        project: project.to_string(),
        max_depth,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| PathError::QueryFailed {
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
    use crate::db::open_mem_db;

    #[test]
    fn test_path_query_cozo_basic() {
        let builder = PathQueryBuilder {
            from_module: "MyApp.Controller".to_string(),
            from_function: "handle_request".to_string(),
            from_arity: None,
            to_module: "MyApp.Repo".to_string(),
            to_function: "insert".to_string(),
            to_arity: None,
            project: "myproject".to_string(),
            max_depth: 10,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify recursive structure with target filtering
        assert!(compiled.contains("trace[depth"));
        assert!(compiled.contains("target_depth"));
        assert!(compiled.contains("$from_module"));
        assert!(compiled.contains("$to_module"));
        assert!(compiled.contains("depth <= min_d"));
    }

    #[test]
    fn test_path_query_cozo_with_arity() {
        let builder = PathQueryBuilder {
            from_module: "MyApp".to_string(),
            from_function: "start".to_string(),
            from_arity: Some(0),
            to_module: "MyApp.DB".to_string(),
            to_function: "query".to_string(),
            to_arity: Some(2),
            project: "myproject".to_string(),
            max_depth: 5,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("callee_arity == $to_arity"));
    }

    #[test]
    fn test_path_query_age() {
        let builder = PathQueryBuilder {
            from_module: "MyApp".to_string(),
            from_function: "start".to_string(),
            from_arity: None,
            to_module: "MyApp.Target".to_string(),
            to_function: "end".to_string(),
            to_arity: None,
            project: "myproject".to_string(),
            max_depth: 5,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("CALLS*1..5"));
        assert!(compiled.contains("source.module"));
        assert!(compiled.contains("target.module"));
    }

    #[test]
    fn test_path_query_parameters() {
        let builder = PathQueryBuilder {
            from_module: "A".to_string(),
            from_function: "a".to_string(),
            from_arity: None,
            to_module: "B".to_string(),
            to_function: "b".to_string(),
            to_arity: Some(1),
            project: "proj".to_string(),
            max_depth: 3,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 6); // from_module, from_function, to_module, to_function, to_arity, project
    }
}
