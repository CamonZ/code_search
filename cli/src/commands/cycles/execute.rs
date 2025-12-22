//! Cycle detection execution and DFS-based cycle reconstruction.

use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde::Serialize;

use super::CyclesCmd;
use crate::commands::Execute;
use db::queries::cycles::find_cycle_edges;

/// A single cycle found in the module dependency graph
#[derive(Debug, Clone, Serialize)]
pub struct Cycle {
    /// Length of the cycle (number of modules)
    pub length: usize,
    /// Ordered path of modules: A → B → C → A
    pub modules: Vec<String>,
}

/// Result of cycle detection
#[derive(Debug, Serialize)]
pub struct CyclesResult {
    /// Total number of distinct cycles found
    pub total_cycles: usize,
    /// Total number of unique modules involved in cycles
    pub modules_in_cycles: usize,
    /// The detected cycles
    pub cycles: Vec<Cycle>,
}

impl Execute for CyclesCmd {
    type Output = CyclesResult;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        // Get cycle edges from the database
        let edges = find_cycle_edges(
            db,
            &self.common.project,
            self.module.as_deref(),
        )?;

        if edges.is_empty() {
            return Ok(CyclesResult {
                total_cycles: 0,
                modules_in_cycles: 0,
                cycles: vec![],
            });
        }

        // Build adjacency list from edges
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_modules = HashSet::new();

        for edge in &edges {
            graph
                .entry(edge.from.clone())
                .or_insert_with(Vec::new)
                .push(edge.to.clone());
            all_modules.insert(edge.from.clone());
            all_modules.insert(edge.to.clone());
        }

        // Find cycles using DFS from each node
        let mut cycles = find_all_cycles(&graph, &all_modules);

        // Filter by max_length if provided
        if let Some(max_len) = self.max_length {
            cycles.retain(|c| c.length <= max_len);
        }

        // Filter by involving module if provided
        if let Some(involving) = &self.involving {
            cycles.retain(|c| c.modules.iter().any(|m| m.contains(involving)));
        }

        // Deduplicate cycles (same modules in different starting positions are the same cycle)
        cycles = deduplicate_cycles(cycles);

        // Count unique modules in cycles
        let modules_in_cycles: HashSet<_> = cycles
            .iter()
            .flat_map(|c| c.modules.iter().cloned())
            .collect();

        Ok(CyclesResult {
            total_cycles: cycles.len(),
            modules_in_cycles: modules_in_cycles.len(),
            cycles,
        })
    }
}

/// Find all cycles starting from each node in the graph using DFS
fn find_all_cycles(graph: &HashMap<String, Vec<String>>, all_modules: &HashSet<String>) -> Vec<Cycle> {
    let mut cycles = Vec::new();

    for start_node in all_modules {
        let found = dfs_find_cycles(graph, start_node, start_node, vec![], &mut HashSet::new());
        cycles.extend(found);
    }

    cycles
}

/// DFS to find cycles starting from a given node
fn dfs_find_cycles(
    graph: &HashMap<String, Vec<String>>,
    current: &str,
    start: &str,
    path: Vec<String>,
    visited: &mut HashSet<String>,
) -> Vec<Cycle> {
    let mut cycles = Vec::new();
    let mut new_path = path.clone();
    new_path.push(current.to_string());

    // If we've revisited the start node and have more than one edge in the path,
    // we've found a cycle
    if current == start && !path.is_empty() {
        // Only report if we haven't already found this cycle
        // (cycles of length > 1 where start != first path node)
        if path.len() > 0 {
            cycles.push(Cycle {
                length: new_path.len() - 1, // Don't count the repeated start node
                modules: path.clone(),
            });
        }
        return cycles;
    }

    // Prevent infinite recursion on the same node in the current path
    if new_path.len() > 1 && path.contains(&current.to_string()) {
        return cycles;
    }

    // Explore neighbors
    if let Some(neighbors) = graph.get(current) {
        for neighbor in neighbors {
            let found = dfs_find_cycles(graph, neighbor, start, new_path.clone(), visited);
            cycles.extend(found);
        }
    }

    cycles
}

/// Remove duplicate cycles (same modules in different orders/rotations)
fn deduplicate_cycles(cycles: Vec<Cycle>) -> Vec<Cycle> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();

    for cycle in cycles {
        // Normalize the cycle representation: sort to get canonical form
        let mut sorted = cycle.modules.clone();
        sorted.sort();
        let canonical = format!("{:?}", sorted);

        if !seen.contains(&canonical) {
            seen.insert(canonical);
            unique.push(cycle);
        }
    }

    // Sort cycles by length and first module for consistent output
    unique.sort_by(|a, b| {
        a.length.cmp(&b.length).then_with(|| {
            a.modules
                .first()
                .cmp(&b.modules.first())
        })
    });

    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_cycles_simple_two_module_cycle() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["A".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        // A-B-A should be detected as one cycle
        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].length, 2);
        assert!(unique[0].modules.contains(&"A".to_string()));
        assert!(unique[0].modules.contains(&"B".to_string()));
    }

    #[test]
    fn test_find_cycles_three_module_cycle() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["C".to_string()]);
        graph.insert("C".to_string(), vec!["A".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].length, 3);
    }

    #[test]
    fn test_find_cycles_no_cycles() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["C".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_deduplicate_cycles() {
        let cycles = vec![
            Cycle {
                length: 2,
                modules: vec!["A".to_string(), "B".to_string()],
            },
            Cycle {
                length: 2,
                modules: vec!["B".to_string(), "A".to_string()],
            },
        ];

        let unique = deduplicate_cycles(cycles);
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn test_max_length_filter() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["C".to_string()]);
        graph.insert("C".to_string(), vec!["A".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].length, 3);

        // Filter to max_length 2
        let filtered: Vec<_> = unique.iter().filter(|c| c.length <= 2).cloned().collect();
        assert_eq!(filtered.len(), 0);
    }
}
