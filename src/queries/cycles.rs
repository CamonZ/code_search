//! Detect circular dependencies between modules using recursive queries.
//!
//! Uses CozoDB's recursive queries to:
//! 1. Build a deduplicated module dependency graph
//! 2. Find reachability (transitive closure)
//! 3. Detect modules that can reach themselves (cycles)
//! 4. Return cycle edges for reconstruction by the command

use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;

use crate::db::{run_query, Params};
use crate::queries::builder::QueryBuilder;

/// Edge in a cycle (from module -> to module)
#[derive(Debug, Clone)]
pub struct CycleEdge {
    pub from: String,
    pub to: String,
}

/// Query builder for cycle detection using transitive closure
#[derive(Debug)]
pub struct CyclesQueryBuilder {
    pub project: String,
    pub module_pattern: Option<String>,
}

impl QueryBuilder for CyclesQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        // Note: module_pattern is applied in Rust post-processing, not in query
        params
    }
}

impl CyclesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        Ok(r#"
        # Build module dependency graph (deduplicated at module level)
        module_deps[from, to] :=
            *calls{project, caller_module: from, callee_module: to},
            project == $project,
            from != to

        # Find reachability (transitive closure) - what modules can be reached from each module
        reaches[from, to] := module_deps[from, to]
        reaches[from, to] := module_deps[from, mid], reaches[mid, to]

        # Find modules in cycles - modules that can reach themselves
        in_cycle[module] := reaches[module, module]

        # Find cycle edges - direct edges between modules that are both in cycles
        cycle_edge[from, to] :=
            module_deps[from, to],
            in_cycle[from],
            in_cycle[to]

        ?[from, to] := cycle_edge[from, to]
        :order from, to
        "#.to_string())
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE doesn't support recursive CTEs in Cypher.
        // Return all module dependencies; cycle detection will be done in Rust.
        // Note: Using "from_mod" and "to_mod" instead of "from" and "to"
        // because those are reserved words in PostgreSQL.
        Ok(r#"MATCH (c:Call)
WHERE c.project = $project
  AND c.caller_module <> c.callee_module
RETURN DISTINCT c.caller_module AS from_mod, c.callee_module AS to_mod"#.to_string())
    }
}

/// Find all module pairs that form cycles
///
/// Returns edges (from, to) where both modules are part of at least one cycle.
pub fn find_cycle_edges(
    db: &dyn DatabaseBackend,
    project: &str,
    module_pattern: Option<&str>,
) -> Result<Vec<CycleEdge>, Box<dyn Error>> {
    use crate::queries::builder::CompiledQuery;

    let builder = CyclesQueryBuilder {
        project: project.to_string(),
        module_pattern: module_pattern.map(|s| s.to_string()),
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params)?;

    // Parse results
    let mut all_edges = Vec::new();

    // Find column indices - AGE uses from_mod/to_mod, Cozo uses from/to
    let from_idx = rows
        .headers
        .iter()
        .position(|h| h == "from" || h == "from_mod")
        .ok_or("Missing 'from' or 'from_mod' column")?;
    let to_idx = rows
        .headers
        .iter()
        .position(|h| h == "to" || h == "to_mod")
        .ok_or("Missing 'to' or 'to_mod' column")?;

    for row in &rows.rows {
        if let (Some(DataValue::Str(from)), Some(DataValue::Str(to))) =
            (row.get(from_idx), row.get(to_idx))
        {
            all_edges.push((from.to_string(), to.to_string()));
        }
    }

    // For AGE backend, we need to detect cycles in Rust
    // For Cozo, the query already returns only cycle edges
    let cycle_edges = if db.backend_name() == "PostgresAge" {
        detect_cycles_in_edges(&all_edges)
    } else {
        all_edges
    };

    // Apply module pattern filter and convert to CycleEdge
    let mut edges = Vec::new();
    for (from, to) in cycle_edges {
        if let Some(pattern) = module_pattern {
            if !from.contains(pattern) && !to.contains(pattern) {
                continue;
            }
        }
        edges.push(CycleEdge { from, to });
    }

    Ok(edges)
}

/// Detect cycles in a set of edges using DFS
/// Returns only edges that are part of at least one cycle
fn detect_cycles_in_edges(edges: &[(String, String)]) -> Vec<(String, String)> {
    use std::collections::{HashMap, HashSet};

    // Build adjacency list
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (from, to) in edges {
        adj.entry(from.as_str()).or_default().push(to.as_str());
    }

    // Find all modules that are reachable from themselves (part of a cycle)
    let mut in_cycle: HashSet<&str> = HashSet::new();

    for start in adj.keys() {
        // DFS to check if we can reach 'start' from 'start'
        let mut visited: HashSet<&str> = HashSet::new();
        let mut stack = vec![*start];

        while let Some(current) = stack.pop() {
            if current == *start && !visited.is_empty() {
                // Found a cycle back to start
                in_cycle.insert(*start);
                break;
            }
            if visited.contains(current) {
                continue;
            }
            visited.insert(current);
            if let Some(neighbors) = adj.get(current) {
                for neighbor in neighbors {
                    stack.push(*neighbor);
                }
            }
        }
    }

    // Return edges where both endpoints are in a cycle
    edges
        .iter()
        .filter(|(from, to)| in_cycle.contains(from.as_str()) && in_cycle.contains(to.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_cycles_query_cozo_basic() {
        let builder = CyclesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify transitive closure structure
        assert!(compiled.contains("module_deps[from, to]"));
        assert!(compiled.contains("reaches[from, to]"));
        assert!(compiled.contains("in_cycle[module]"));
        assert!(compiled.contains("reaches[module, module]")); // Self-reachability
        assert!(compiled.contains("cycle_edge[from, to]"));
    }

    #[test]
    fn test_cycles_query_cozo_self_exclusion() {
        let builder = CyclesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify self-references are excluded in dependency graph
        assert!(compiled.contains("from != to"));
    }

    #[test]
    fn test_cycles_query_age() {
        let builder = CyclesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE version returns all edges; cycle detection is done in Rust
        assert!(compiled.contains("MATCH (c:Call)"));
        assert!(compiled.contains("c.caller_module <> c.callee_module"));
        assert!(compiled.contains("RETURN DISTINCT"));
    }

    #[test]
    fn test_cycles_query_parameters() {
        let builder = CyclesQueryBuilder {
            project: "proj".to_string(),
            module_pattern: Some("MyApp".to_string()),
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 1); // Only project, module_pattern is post-filter
        assert!(params.contains_key("project"));
    }
}
