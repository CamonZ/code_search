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
        Ok(r#"
        -- Find all modules that are part of a cycle (can reach themselves)
        WITH RECURSIVE cycle_modules AS (
            -- Base: all edges
            SELECT DISTINCT from_mod, to_mod
            FROM module_deps
            WHERE project = $project

            UNION ALL

            -- Recursive: paths between modules
            SELECT m.from_mod, d.to_mod
            FROM cycle_modules m
            JOIN module_deps d ON m.to_mod = d.from_mod
            WHERE d.project = $project
        ),
        -- Identify modules that can reach themselves
        in_cycle AS (
            SELECT DISTINCT from_mod as module
            FROM cycle_modules
            WHERE from_mod = to_mod
        ),
        -- Find edges between modules in cycles
        cycle_edges AS (
            SELECT DISTINCT d.from_mod as from, d.to_mod as to
            FROM module_deps d
            WHERE d.project = $project
              AND d.from_mod IN (SELECT module FROM in_cycle)
              AND d.to_mod IN (SELECT module FROM in_cycle)
        )
        SELECT from, to FROM cycle_edges ORDER BY from, to
        "#.to_string())
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
    let builder = CyclesQueryBuilder {
        project: project.to_string(),
        module_pattern: module_pattern.map(|s| s.to_string()),
    };

    let compiled_script = builder.compile(db)?;
    let params = builder.parameters();

    let rows = run_query(db, &compiled_script, params)?;

    // Parse results
    let mut edges = Vec::new();

    // Find column indices
    let from_idx = rows
        .headers
        .iter()
        .position(|h| h == "from")
        .ok_or("Missing 'from' column")?;
    let to_idx = rows
        .headers
        .iter()
        .position(|h| h == "to")
        .ok_or("Missing 'to' column")?;

    for row in &rows.rows {
        if let (Some(DataValue::Str(from)), Some(DataValue::Str(to))) =
            (row.get(from_idx), row.get(to_idx))
        {
            // Apply module pattern filter if provided
            if let Some(pattern) = module_pattern {
                if !from.contains(pattern) && !to.contains(pattern) {
                    continue;
                }
            }
            edges.push(CycleEdge {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
    }

    Ok(edges)
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

        assert!(compiled.contains("MATCH") || compiled.contains("WITH"));
        // AGE cycle detection pattern - should mention cycles or recursion
        assert!(compiled.contains("cycle"));
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
