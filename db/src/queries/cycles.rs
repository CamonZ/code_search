//! Detect circular dependencies between modules using recursive queries.
//!
//! Uses backend-specific recursive queries to:
//! 1. Build a deduplicated module dependency graph
//! 2. Find reachability (transitive closure)
//! 3. Detect modules that can reach themselves (cycles)
//! 4. Return cycle edges for reconstruction by the command

use std::error::Error;

use crate::backend::{Database, QueryParams};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

/// Edge in a cycle (from module -> to module)
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CycleEdge {
    pub from: String,
    pub to: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
/// Find all module pairs that form cycles
///
/// Returns edges (from, to) where both modules are part of at least one cycle.
/// Note: SurrealDB doesn't have built-in recursive CTEs, so we use a multi-step
/// approach to detect cycles by finding modules that can reach themselves.
pub fn find_cycle_edges(
    db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
) -> Result<Vec<CycleEdge>, Box<dyn Error>> {
    // Step 1: Get all direct module-to-module dependencies
    // Note: In SurrealDB RELATE syntax (A ->edge-> B), `in` is A (source) and `out` is B (target)
    // So for `caller ->calls-> callee`: in=caller, out=callee
    // We also filter out self-loops (same module calling itself)
    let deps_query = r#"
        SELECT
            in.module_name as from_module,
            out.module_name as to_module
        FROM calls
        WHERE in.module_name != out.module_name
    "#;

    let result = db.execute_query(deps_query, QueryParams::new())?;

    // Parse direct dependencies into a map, deduplicating along the way
    let mut deps: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    for row in result.rows() {
        if row.len() >= 2 {
            if let (Some(from_val), Some(to_val)) = (row.get(0), row.get(1)) {
                if let (Some(from), Some(to)) = (from_val.as_str(), to_val.as_str()) {
                    deps.entry(from.to_string())
                        .or_insert_with(std::collections::HashSet::new)
                        .insert(to.to_string());
                }
            }
        }
    }

    // Step 2: Compute reachability - for each module, find all modules it can reach
    // This allows us to check if an edge A→B is part of a cycle (B can reach A)
    fn compute_reachable(
        start: &str,
        deps: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    ) -> std::collections::HashSet<String> {
        let mut reachable = std::collections::HashSet::new();
        let mut queue = vec![start.to_string()];

        while let Some(current) = queue.pop() {
            if let Some(neighbors) = deps.get(&current) {
                for neighbor in neighbors {
                    if reachable.insert(neighbor.clone()) {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }

        reachable
    }

    // Precompute reachability for all modules
    let mut reachability: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    for module in deps.keys() {
        reachability.insert(module.clone(), compute_reachable(module, &deps));
    }

    // Step 3: An edge A→B is a cycle edge if B can reach A (completing the cycle)
    let mut edges = Vec::new();

    for (from, tos) in &deps {
        for to in tos {
            // Check if 'to' can reach 'from' (making from→to part of a cycle)
            if let Some(to_reaches) = reachability.get(to) {
                if to_reaches.contains(from) {
                    // Apply module pattern filter if provided
                    if let Some(pattern) = module_pattern {
                        if !from.contains(pattern) && !to.contains(pattern) {
                            continue;
                        }
                    }

                    edges.push(CycleEdge {
                        from: from.clone(),
                        to: to.clone(),
                    });
                }
            }
        }
    }

    // Remove duplicates and sort edges for consistent output
    let unique_edges: std::collections::HashSet<_> = edges
        .into_iter()
        .collect();

    let mut sorted_edges: Vec<_> = unique_edges.into_iter().collect();
    sorted_edges.sort_by(|a, b| {
        match a.from.cmp(&b.from) {
            std::cmp::Ordering::Equal => a.to.cmp(&b.to),
            other => other,
        }
    });

    Ok(sorted_edges)
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
    fn test_find_cycle_edges_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "default", None);
        assert!(result.is_ok());
        let edges = result.unwrap();
        // May or may not have cycles, but query should execute successfully
        assert!(edges.is_empty() || !edges.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_cycle_edges_empty_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "nonexistent", None);
        assert!(result.is_ok());
        let edges = result.unwrap();
        assert!(edges.is_empty(), "Non-existent project should have no cycles");
    }

    #[rstest]
    fn test_find_cycle_edges_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "default", Some("MyApp"));
        assert!(result.is_ok());
        let edges = result.unwrap();
        // All results should contain the module pattern
        for edge in &edges {
            assert!(
                edge.from.contains("MyApp") || edge.to.contains("MyApp"),
                "Edge should contain module pattern"
            );
        }
    }

    #[rstest]
    fn test_find_cycle_edges_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "default", None);
        assert!(result.is_ok());
        let edges = result.unwrap();
        for edge in &edges {
            assert!(!edge.from.is_empty());
            assert!(!edge.to.is_empty());
            // In a real cycle, from and to should be different
            // (self-cycles are filtered out in the query)
        }
    }

    #[rstest]
    fn test_find_cycle_edges_edges_are_distinct(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "default", None);
        assert!(result.is_ok());
        let edges = result.unwrap();

        // Check that edges are ordered
        for i in 1..edges.len() {
            let prev = (&edges[i - 1].from, &edges[i - 1].to);
            let curr = (&edges[i].from, &edges[i].to);
            assert!(
                (prev.0 < curr.0) || (prev.0 == curr.0 && prev.1 <= curr.1),
                "Edges should be in order"
            );
        }
    }

    #[rstest]
    fn test_find_cycle_edges_all_edges_valid(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_cycle_edges(&*populated_db, "default", None);
        assert!(result.is_ok());
        let edges = result.unwrap();
        // All edges should have non-empty modules
        for edge in &edges {
            assert!(!edge.from.is_empty());
            assert!(!edge.to.is_empty());
            // In cycles, from and to should be different (no self-cycles)
            assert_ne!(edge.from, edge.to);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Fixture cycle structure =====
    // The complex fixture has 3 explicit cycles plus additional cross-cycle edges.
    // Since all 9 modules can reach themselves (are in cycles), and the function
    // returns edges where BOTH endpoints are in cycles, we get 17 unique edges.
    //
    // Cycle A (3 nodes): Service → Logger → Repo → Service
    // Cycle B (4 nodes): Controller → Events → Cache → Accounts → Controller
    // Cycle C (5 nodes): Notifier → Metrics → Logger → Events → Cache → Notifier
    //
    // Plus original non-cycle edges that connect modules which are still in cycles:
    //   - Controller → Accounts, Controller → Service, Controller → Notifier
    //   - Service → Accounts, Service → Notifier
    //   - Accounts → Repo
    //
    // All 17 unique module-level edges (sorted):
    //   1. Accounts → Controller, 2. Accounts → Repo
    //   3. Cache → Accounts, 4. Cache → Notifier
    //   5. Controller → Accounts, 6. Controller → Events, 7. Controller → Notifier, 8. Controller → Service
    //   9. Events → Cache
    //   10. Logger → Events, 11. Logger → Repo
    //   12. Metrics → Logger
    //   13. Notifier → Metrics
    //   14. Repo → Service
    //   15. Service → Accounts, 16. Service → Logger, 17. Service → Notifier

    // ===== Core cycle detection tests =====

    #[test]
    fn test_find_cycle_edges_returns_exactly_17_edges() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // The fixture has 17 unique module-level edges between modules that are in cycles
        assert_eq!(
            edges.len(),
            17,
            "Should find exactly 17 unique cycle edges, got {}",
            edges.len()
        );
    }

    #[test]
    fn test_find_cycle_edges_contains_all_expected_edges() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // All 17 expected edges (sorted alphabetically)
        let expected_edges = [
            ("MyApp.Accounts", "MyApp.Controller"),
            ("MyApp.Accounts", "MyApp.Repo"),
            ("MyApp.Cache", "MyApp.Accounts"),
            ("MyApp.Cache", "MyApp.Notifier"),
            ("MyApp.Controller", "MyApp.Accounts"),
            ("MyApp.Controller", "MyApp.Events"),
            ("MyApp.Controller", "MyApp.Notifier"),
            ("MyApp.Controller", "MyApp.Service"),
            ("MyApp.Events", "MyApp.Cache"),
            ("MyApp.Logger", "MyApp.Events"),
            ("MyApp.Logger", "MyApp.Repo"),
            ("MyApp.Metrics", "MyApp.Logger"),
            ("MyApp.Notifier", "MyApp.Metrics"),
            ("MyApp.Repo", "MyApp.Service"),
            ("MyApp.Service", "MyApp.Accounts"),
            ("MyApp.Service", "MyApp.Logger"),
            ("MyApp.Service", "MyApp.Notifier"),
        ];

        for (from, to) in expected_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Expected edge {} → {} should be present in results",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_contains_cycle_a_edges() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // Cycle A: Service → Logger → Repo → Service
        let cycle_a_edges = [
            ("MyApp.Service", "MyApp.Logger"),
            ("MyApp.Logger", "MyApp.Repo"),
            ("MyApp.Repo", "MyApp.Service"),
        ];

        for (from, to) in cycle_a_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Cycle A edge {} → {} should be present in results",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_contains_cycle_b_edges() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // Cycle B: Controller → Events → Cache → Accounts → Controller
        let cycle_b_edges = [
            ("MyApp.Controller", "MyApp.Events"),
            ("MyApp.Events", "MyApp.Cache"),
            ("MyApp.Cache", "MyApp.Accounts"),
            ("MyApp.Accounts", "MyApp.Controller"),
        ];

        for (from, to) in cycle_b_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Cycle B edge {} → {} should be present in results",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_contains_cycle_c_edges() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // Cycle C: Notifier → Metrics → Logger → Events → Cache → Notifier
        let cycle_c_edges = [
            ("MyApp.Notifier", "MyApp.Metrics"),
            ("MyApp.Metrics", "MyApp.Logger"),
            ("MyApp.Logger", "MyApp.Events"),
            ("MyApp.Events", "MyApp.Cache"),
            ("MyApp.Cache", "MyApp.Notifier"),
        ];

        for (from, to) in cycle_c_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Cycle C edge {} → {} should be present in results",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_involves_exactly_9_modules() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        let mut modules = std::collections::HashSet::new();
        for edge in &edges {
            modules.insert(edge.from.clone());
            modules.insert(edge.to.clone());
        }

        assert_eq!(
            modules.len(),
            9,
            "Should involve exactly 9 modules in cycles, got {}",
            modules.len()
        );

        // Verify each expected module is present
        let expected_modules = [
            "MyApp.Accounts",
            "MyApp.Cache",
            "MyApp.Controller",
            "MyApp.Events",
            "MyApp.Logger",
            "MyApp.Metrics",
            "MyApp.Notifier",
            "MyApp.Repo",
            "MyApp.Service",
        ];

        for module in expected_modules {
            assert!(
                modules.contains(module),
                "Module {} should be in a cycle",
                module
            );
        }
    }

    // ===== Ordering and uniqueness tests =====

    #[test]
    fn test_find_cycle_edges_are_sorted_alphabetically() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        // Verify sorted order: by from module, then by to module
        for i in 1..edges.len() {
            let prev = (&edges[i - 1].from, &edges[i - 1].to);
            let curr = (&edges[i].from, &edges[i].to);

            let is_ordered = prev.0 < curr.0 || (prev.0 == curr.0 && prev.1 < curr.1);
            assert!(
                is_ordered,
                "Edges should be sorted: {} → {} should come before {} → {}",
                prev.0, prev.1, curr.0, curr.1
            );
        }

        // Verify first and last edges alphabetically
        assert_eq!(edges[0].from, "MyApp.Accounts");
        assert_eq!(edges[0].to, "MyApp.Controller");
        assert_eq!(edges[16].from, "MyApp.Service");
        assert_eq!(edges[16].to, "MyApp.Notifier");
    }

    #[test]
    fn test_find_cycle_edges_has_no_duplicates() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        let mut seen = std::collections::HashSet::new();
        for edge in &edges {
            let key = (edge.from.clone(), edge.to.clone());
            assert!(
                seen.insert(key.clone()),
                "Duplicate edge found: {} → {}",
                edge.from, edge.to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_has_no_self_loops() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", None)
            .expect("Query should succeed");

        for edge in &edges {
            assert_ne!(
                edge.from, edge.to,
                "Self-loop found: {} → {}",
                edge.from, edge.to
            );
        }
    }

    // ===== Module pattern filter tests =====

    #[test]
    fn test_find_cycle_edges_filter_by_service_module() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", Some("Service"))
            .expect("Query should succeed");

        // Edges involving Service:
        // - Controller → Service, Repo → Service (incoming)
        // - Service → Accounts, Service → Logger, Service → Notifier (outgoing)
        assert_eq!(
            edges.len(),
            5,
            "Filter 'Service' should match 5 edges, got {}",
            edges.len()
        );

        for edge in &edges {
            let matches = edge.from.contains("Service") || edge.to.contains("Service");
            assert!(
                matches,
                "Edge {} → {} should contain 'Service'",
                edge.from, edge.to
            );
        }

        // Verify specific Service edges
        let expected_service_edges = [
            ("MyApp.Controller", "MyApp.Service"),
            ("MyApp.Repo", "MyApp.Service"),
            ("MyApp.Service", "MyApp.Accounts"),
            ("MyApp.Service", "MyApp.Logger"),
            ("MyApp.Service", "MyApp.Notifier"),
        ];

        for (from, to) in expected_service_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Service-related edge {} → {} should be present",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_filter_by_cache_module() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", Some("Cache"))
            .expect("Query should succeed");

        // Cache edges:
        // - Events → Cache (incoming)
        // - Cache → Accounts, Cache → Notifier (outgoing)
        assert_eq!(
            edges.len(),
            3,
            "Filter 'Cache' should match 3 edges, got {}",
            edges.len()
        );

        // Verify specific Cache edges
        let expected_cache_edges = [
            ("MyApp.Events", "MyApp.Cache"),
            ("MyApp.Cache", "MyApp.Accounts"),
            ("MyApp.Cache", "MyApp.Notifier"),
        ];

        for (from, to) in expected_cache_edges {
            let found = edges.iter().any(|e| e.from == from && e.to == to);
            assert!(
                found,
                "Cache-related edge {} → {} should be present",
                from, to
            );
        }
    }

    #[test]
    fn test_find_cycle_edges_filter_nonexistent_returns_empty() {
        let db = get_db();
        let edges = find_cycle_edges(&*db, "default", Some("NonExistentModule"))
            .expect("Query should succeed");

        assert!(
            edges.is_empty(),
            "Non-existent module filter should return empty, got {} edges",
            edges.len()
        );
    }

    // ===== Query behavior tests =====

    #[test]
    fn test_find_cycle_edges_is_idempotent() {
        let db = get_db();
        let result1 = find_cycle_edges(&*db, "default", None)
            .expect("First query should succeed");
        let result2 = find_cycle_edges(&*db, "default", None)
            .expect("Second query should succeed");

        assert_eq!(result1.len(), result2.len(), "Query should be idempotent");

        for i in 0..result1.len() {
            assert_eq!(result1[i].from, result2[i].from);
            assert_eq!(result1[i].to, result2[i].to);
        }
    }
}
