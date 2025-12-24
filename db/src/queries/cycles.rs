//! Detect circular dependencies between modules using recursive queries.
//!
//! Uses CozoDB's recursive queries to:
//! 1. Build a deduplicated module dependency graph
//! 2. Find reachability (transitive closure)
//! 3. Detect modules that can reach themselves (cycles)
//! 4. Return cycle edges for reconstruction by the command

use std::error::Error;

use crate::backend::{Database, QueryParams};
use crate::db::run_query;

/// Edge in a cycle (from module -> to module)
#[derive(Debug, Clone)]
pub struct CycleEdge {
    pub from: String,
    pub to: String,
}

/// Find all module pairs that form cycles
///
/// Returns edges (from, to) where both modules are part of at least one cycle.
pub fn find_cycle_edges(
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
) -> Result<Vec<CycleEdge>, Box<dyn Error>> {
    // Build the recursive query for cycle detection
    let script = r#"
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
    "#.to_string();

    let params = QueryParams::new()
        .with_str("project", project);

    let result = run_query(db, &script, params)?;

    // Parse results
    let mut edges = Vec::new();

    // Find column indices
    let from_idx = result
        .headers()
        .iter()
        .position(|h| h == "from")
        .ok_or("Missing 'from' column")?;
    let to_idx = result
        .headers()
        .iter()
        .position(|h| h == "to")
        .ok_or("Missing 'to' column")?;

    for row in result.rows() {
        if let (Some(from_val), Some(to_val)) =
            (row.get(from_idx), row.get(to_idx))
        {
            if let (Some(from), Some(to)) = (from_val.as_str(), to_val.as_str()) {
                // Apply module pattern filter if provided
                if let Some(pattern) = module_pattern
                    && !from.contains(pattern) && !to.contains(pattern) {
                        continue;
                    }
                edges.push(CycleEdge {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        }
    }

    Ok(edges)
}
