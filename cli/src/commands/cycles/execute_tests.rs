//! Execute tests for cycles command.

#[cfg(test)]
mod tests {
    use super::super::CyclesCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_cycles_basic(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.total_cycles > 0, "Should find cycles in the fixture");
        assert!(result.modules_in_cycles > 0, "Should have modules in cycles");
        assert!(!result.cycles.is_empty(), "Should have cycle entries");
    }

    #[rstest]
    fn test_cycles_finds_expected_cycle_paths(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // The fixture has cycles; verify each cycle's modules form a valid path
        for cycle in &result.cycles {
            assert!(cycle.length >= 2, "Cycles must have at least 2 modules");
            assert_eq!(
                cycle.modules.len(),
                cycle.length,
                "Cycle modules vec length should match cycle.length"
            );
        }
    }

    #[rstest]
    fn test_cycles_no_match_returns_empty(populated_db: Box<dyn db::backend::Database>) {
        let cmd = CyclesCmd {
            module: Some("NonExistentModuleName12345".to_string()),
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_cycles, 0);
        assert_eq!(result.modules_in_cycles, 0);
        assert!(result.cycles.is_empty());
    }

    // =========================================================================
    // max_length boundary tests (kills execute.rs:68 <= replaced with >)
    // =========================================================================

    /// Tests that max_length uses <= (inclusive). A cycle with length == max_length
    /// should be included.
    /// Kills: execute.rs:68 <= replaced with > (cycle at exact boundary excluded)
    #[rstest]
    fn test_max_length_boundary_inclusive(populated_db: Box<dyn db::backend::Database>) {
        // First, discover all cycles without a length filter
        let discover_cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let all_results = discover_cmd
            .execute(&*populated_db)
            .expect("Execute should succeed");
        assert!(!all_results.cycles.is_empty(), "Should have cycles");

        // Find the shortest cycle length
        let shortest_length = all_results
            .cycles
            .iter()
            .map(|c| c.length)
            .min()
            .unwrap();

        // With max_length == shortest cycle length, we should still include those cycles
        let at_boundary_cmd = CyclesCmd {
            module: None,
            max_length: Some(shortest_length),
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let at_boundary = at_boundary_cmd
            .execute(&*populated_db)
            .expect("Execute should succeed");

        assert!(
            !at_boundary.cycles.is_empty(),
            "Cycles of length {} should be included with max_length={} (<=)",
            shortest_length,
            shortest_length
        );

        // All returned cycles should have length <= shortest_length
        for cycle in &at_boundary.cycles {
            assert!(
                cycle.length <= shortest_length,
                "Cycle length {} should be <= max_length {}",
                cycle.length,
                shortest_length
            );
        }

        // With max_length == shortest_length - 1, those cycles should be excluded
        if shortest_length > 1 {
            let below_boundary_cmd = CyclesCmd {
                module: None,
                max_length: Some(shortest_length - 1),
                involving: None,
                common: CommonArgs {
                    regex: false,
                    limit: 100,
                },
            };
            let below_boundary = below_boundary_cmd
                .execute(&*populated_db)
                .expect("Execute should succeed");

            // If all cycles have the same length, this should be empty
            let cycles_at_shortest = all_results
                .cycles
                .iter()
                .filter(|c| c.length == shortest_length)
                .count();
            if cycles_at_shortest == all_results.cycles.len() {
                assert!(
                    below_boundary.cycles.is_empty(),
                    "No cycles should pass with max_length below the shortest cycle"
                );
            }
        }
    }

    // =========================================================================
    // dfs_find_cycles boundary tests (kills execute.rs:131 > replaced with ==, <, >=)
    // =========================================================================

    /// Tests that the > comparison on line 131 works correctly by verifying
    /// cycles are found in a graph that requires the DFS to traverse properly.
    ///
    /// Line 131: `if new_path.len() > 1 && path.contains(&current.to_string())`
    ///
    /// This condition prevents infinite recursion by blocking re-visits to nodes
    /// already in the path. The > 1 check ensures the start node itself is not
    /// blocked on the first step.
    ///
    /// Kills: execute.rs:131 > replaced with == (would block at len==2 instead of >1)
    /// Kills: execute.rs:131 > replaced with < (would block at len<1, i.e. never blocks, infinite loop)
    /// Kills: execute.rs:131 > replaced with >= (would block at len==1, preventing any DFS from progressing)
    #[rstest]
    fn test_dfs_finds_cycles_with_correct_path_guard(
        populated_db: Box<dyn db::backend::Database>,
    ) {
        let cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // With the correct > comparison, DFS should find cycles.
        // With >= (mutant), new_path.len() >= 1 is always true after push,
        // so even the first step would be blocked, finding 0 cycles.
        // With == (mutant), new_path.len() == 1 is only true at start,
        // so the guard would never trigger for longer paths, causing infinite recursion.
        assert!(
            result.total_cycles > 0,
            "DFS should find cycles with correct > guard on path length"
        );

        // Verify we find cycles of length > 2 (which requires DFS to go deeper)
        // This kills the == mutant: with ==, the guard at new_path.len() == 1
        // would not stop re-visits at depth > 1, potentially causing issues
        let has_longer_cycles = result.cycles.iter().any(|c| c.length > 2);
        assert!(
            has_longer_cycles,
            "Should find cycles longer than 2, which requires proper DFS traversal"
        );
    }

    // =========================================================================
    // Unit tests for internal functions (directly test find_all_cycles and dfs_find_cycles)
    // =========================================================================

    /// Test dfs_find_cycles with a graph that has a 2-node cycle
    /// to verify the > 1 boundary condition at line 131.
    #[test]
    fn test_dfs_two_node_cycle_path_len_boundary() {
        use super::super::execute::{find_all_cycles, deduplicate_cycles};
        use std::collections::{HashMap, HashSet};

        // A -> B -> A (2-node cycle)
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["A".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        // Should find exactly 1 unique cycle: A -> B -> A
        assert_eq!(unique.len(), 1, "Should find exactly 1 cycle in A<->B graph");
        assert_eq!(unique[0].length, 2, "Cycle should have length 2");
    }

    /// Test that find_all_cycles properly handles a chain with a cycle at the end.
    /// This exercises the path guard at depth > 1 where new_path.len() > 1
    /// should prevent re-visiting non-start nodes.
    #[test]
    fn test_dfs_chain_with_terminal_cycle() {
        use super::super::execute::{find_all_cycles, deduplicate_cycles};
        use std::collections::{HashMap, HashSet};

        // A -> B -> C -> B (cycle at B-C, not involving start node A)
        // But also B -> C -> B is a cycle when starting from B
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["C".to_string()]);
        graph.insert("C".to_string(), vec!["B".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        // Should find the B-C cycle
        assert_eq!(unique.len(), 1, "Should find exactly 1 unique cycle (B<->C)");
        assert_eq!(unique[0].length, 2);
        assert!(
            unique[0].modules.contains(&"B".to_string())
                && unique[0].modules.contains(&"C".to_string()),
            "Cycle should contain B and C"
        );
    }

    /// Test a 3-node cycle to verify DFS traverses multiple steps correctly.
    /// This kills the == mutant on line 131 which would only guard at depth 1.
    #[test]
    fn test_dfs_three_node_cycle() {
        use super::super::execute::{find_all_cycles, deduplicate_cycles};
        use std::collections::{HashMap, HashSet};

        // A -> B -> C -> A
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

        assert_eq!(unique.len(), 1, "Should find exactly 1 three-node cycle");
        assert_eq!(unique[0].length, 3, "Cycle should have length 3");
    }

    /// Test a graph with overlapping cycles to verify the path guard allows
    /// proper exploration without infinite recursion.
    #[test]
    fn test_dfs_overlapping_cycles() {
        use super::super::execute::{find_all_cycles, deduplicate_cycles};
        use std::collections::{HashMap, HashSet};

        // Two overlapping cycles sharing node B:
        // A -> B -> A (cycle 1)
        // B -> C -> B (cycle 2)
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["A".to_string(), "C".to_string()]);
        graph.insert("C".to_string(), vec!["B".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        assert_eq!(unique.len(), 2, "Should find 2 unique cycles");
    }

    /// Test that an acyclic graph produces no cycles and DFS terminates.
    #[test]
    fn test_dfs_acyclic_graph_no_cycles() {
        use super::super::execute::{find_all_cycles, deduplicate_cycles};
        use std::collections::{HashMap, HashSet};

        // A -> B -> C -> D (no cycle)
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["C".to_string()]);
        graph.insert("C".to_string(), vec!["D".to_string()]);

        let mut modules = HashSet::new();
        modules.insert("A".to_string());
        modules.insert("B".to_string());
        modules.insert("C".to_string());
        modules.insert("D".to_string());

        let cycles = find_all_cycles(&graph, &modules);
        let unique = deduplicate_cycles(cycles);

        assert!(unique.is_empty(), "Acyclic graph should produce no cycles");
    }

    // =========================================================================
    // run() integration tests (kills mod.rs:42 mutants)
    // =========================================================================

    /// Tests that run() produces non-empty, correct output.
    /// Kills: mod.rs:42 run() -> Ok(String::new()) and Ok("xyzzy")
    #[rstest]
    fn test_run_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Must not be empty (kills -> Ok(String::new()))
        assert!(!output.is_empty(), "run() should produce non-empty output");
        // Must contain expected header (kills -> Ok("xyzzy"))
        assert!(
            output.contains("Circular Dependencies"),
            "Output should contain 'Circular Dependencies', got: {}",
            output
        );
        // Verify it contains actual cycle data
        assert!(
            output.contains("cycle(s)"),
            "Output should contain cycle count summary"
        );
    }

    /// Tests run() with empty results to ensure even empty output is correct.
    #[rstest]
    fn test_run_empty_produces_correct_output(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CyclesCmd {
            module: Some("NonExistentModuleName12345".to_string()),
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        assert!(
            output.contains("No circular dependencies found"),
            "Empty output should show no-cycles message, got: {}",
            output
        );
    }

    /// Tests run() with JSON format to kill mutants via format path.
    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        // Must be valid JSON (kills String::new() and "xyzzy")
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");
        assert!(parsed["total_cycles"].as_u64().unwrap() > 0);
        assert!(parsed["modules_in_cycles"].as_u64().unwrap() > 0);
    }

    // =========================================================================
    // involving filter test
    // =========================================================================

    #[rstest]
    fn test_involving_filter(populated_db: Box<dyn db::backend::Database>) {
        // First get all cycles
        let all_cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: None,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let all_result = all_cmd.execute(&*populated_db).expect("Execute should succeed");
        assert!(!all_result.cycles.is_empty(), "Should have cycles");

        // Pick a module name from the first cycle
        let target_module = &all_result.cycles[0].modules[0];

        // Filter to only cycles involving that module
        let filtered_cmd = CyclesCmd {
            module: None,
            max_length: None,
            involving: Some(target_module.clone()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let filtered_result = filtered_cmd
            .execute(&*populated_db)
            .expect("Execute should succeed");

        // All returned cycles should involve the target module
        for cycle in &filtered_result.cycles {
            assert!(
                cycle.modules.iter().any(|m| m.contains(target_module.as_str())),
                "Cycle {:?} should involve {}",
                cycle.modules,
                target_module
            );
        }
    }
}
