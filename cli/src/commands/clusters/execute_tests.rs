//! Execute tests for clusters command.

#[cfg(test)]
mod tests {
    use super::super::ClustersCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};

    crate::surreal_fixture! {
        fixture_name: populated_db,
    }

    // =========================================================================
    // Helper to build ClustersCmd
    // =========================================================================

    fn clusters_cmd(depth: usize, module: Option<&str>, show_deps: bool) -> ClustersCmd {
        ClustersCmd {
            depth,
            show_dependencies: show_deps,
            module: module.map(|s| s.to_string()),
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        }
    }

    // =========================================================================
    // Depth 1: all modules collapse to "MyApp" -- single cluster
    // =========================================================================

    #[rstest]
    fn test_depth1_single_cluster(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.depth, 1);
        assert_eq!(result.total_clusters, 1);
        assert_eq!(result.clusters.len(), 1);

        let cluster = &result.clusters[0];
        assert_eq!(cluster.namespace, "MyApp");
    }

    #[rstest]
    fn test_depth1_all_calls_internal(populated_db: Box<dyn db::backend::Database>) {
        // At depth 1, all 20 inter-module calls are internal to "MyApp"
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        assert_eq!(cluster.internal_calls, 20, "All 20 inter-module calls should be internal at depth 1");
        assert_eq!(cluster.outgoing_calls, 0);
        assert_eq!(cluster.incoming_calls, 0);
    }

    #[rstest]
    fn test_depth1_cohesion_is_one(populated_db: Box<dyn db::backend::Database>) {
        // cohesion = internal / (internal + outgoing + incoming) = 20 / 20 = 1.0
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        assert!((cluster.cohesion - 1.0).abs() < f64::EPSILON,
            "Cohesion should be 1.0 when all calls are internal, got {}", cluster.cohesion);
    }

    #[rstest]
    fn test_depth1_instability_is_zero(populated_db: Box<dyn db::backend::Database>) {
        // instability = outgoing / (incoming + outgoing) = 0 / 0 = 0.0 (default)
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        assert!((cluster.instability - 0.0).abs() < f64::EPSILON,
            "Instability should be 0.0, got {}", cluster.instability);
    }

    #[rstest]
    fn test_depth1_module_count(populated_db: Box<dyn db::backend::Database>) {
        // All 9 modules should be in the single namespace
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        assert_eq!(cluster.module_count, 9, "Should have 9 modules in the MyApp namespace");
    }

    // =========================================================================
    // Depth 2: each module is its own namespace -- 9 clusters, all cross-ns
    // =========================================================================

    #[rstest]
    fn test_depth2_cluster_count(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.depth, 2);
        assert_eq!(result.total_clusters, 9);
        assert_eq!(result.clusters.len(), 9);
    }

    #[rstest]
    fn test_depth2_no_internal_calls(populated_db: Box<dyn db::backend::Database>) {
        // At depth 2, each module is its own namespace, so no calls are internal
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for cluster in &result.clusters {
            assert_eq!(cluster.internal_calls, 0,
                "No internal calls expected at depth 2, but {} has {}",
                cluster.namespace, cluster.internal_calls);
        }
    }

    #[rstest]
    fn test_depth2_cohesion_is_zero(populated_db: Box<dyn db::backend::Database>) {
        // With 0 internal calls, cohesion = 0 / total = 0.0 for all clusters
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for cluster in &result.clusters {
            assert!((cluster.cohesion - 0.0).abs() < f64::EPSILON,
                "Cohesion should be 0.0 at depth 2, but {} has {}",
                cluster.namespace, cluster.cohesion);
        }
    }

    #[rstest]
    fn test_depth2_controller_calls(populated_db: Box<dyn db::backend::Database>) {
        // Controller: outgoing=5 (Accounts*2, Service*1, Notifier*1, Events*1), incoming=1 (from Accounts)
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let controller = result.clusters.iter().find(|c| c.namespace == "MyApp.Controller").unwrap();
        assert_eq!(controller.outgoing_calls, 5, "Controller outgoing");
        assert_eq!(controller.incoming_calls, 1, "Controller incoming");
    }

    #[rstest]
    fn test_depth2_accounts_calls(populated_db: Box<dyn db::backend::Database>) {
        // Accounts: outgoing=3 (Repo*2, Controller*1), incoming=4 (Controller*2, Service*1, Cache*1)
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let accounts = result.clusters.iter().find(|c| c.namespace == "MyApp.Accounts").unwrap();
        assert_eq!(accounts.outgoing_calls, 3, "Accounts outgoing");
        assert_eq!(accounts.incoming_calls, 4, "Accounts incoming");
    }

    #[rstest]
    fn test_depth2_instability_controller(populated_db: Box<dyn db::backend::Database>) {
        // Controller: instability = outgoing / (incoming + outgoing) = 5 / (1 + 5) = 5/6
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let controller = result.clusters.iter().find(|c| c.namespace == "MyApp.Controller").unwrap();
        let expected = 5.0 / 6.0;
        assert!((controller.instability - expected).abs() < 0.01,
            "Controller instability should be ~{:.4}, got {:.4}", expected, controller.instability);
    }

    #[rstest]
    fn test_depth2_instability_repo(populated_db: Box<dyn db::backend::Database>) {
        // Repo: outgoing=1, incoming=3, instability = 1/4 = 0.25 (stable, depended upon)
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let repo = result.clusters.iter().find(|c| c.namespace == "MyApp.Repo").unwrap();
        assert_eq!(repo.outgoing_calls, 1);
        assert_eq!(repo.incoming_calls, 3);
        assert!((repo.instability - 0.25).abs() < 0.01,
            "Repo instability should be 0.25, got {}", repo.instability);
    }

    #[rstest]
    fn test_depth2_metrics_calls(populated_db: Box<dyn db::backend::Database>) {
        // Metrics: outgoing=1 (Logger), incoming=1 (Notifier)
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let metrics = result.clusters.iter().find(|c| c.namespace == "MyApp.Metrics").unwrap();
        assert_eq!(metrics.outgoing_calls, 1);
        assert_eq!(metrics.incoming_calls, 1);
        // instability = 1 / 2 = 0.5
        assert!((metrics.instability - 0.5).abs() < 0.01);
    }

    // =========================================================================
    // Sort ordering: cohesion descending, then internal_calls descending
    // =========================================================================

    #[rstest]
    fn test_depth2_sorted_by_cohesion(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // All have cohesion 0.0 at depth 2, so they should be sorted by internal_calls (all 0)
        for window in result.clusters.windows(2) {
            assert!(window[0].cohesion >= window[1].cohesion,
                "Clusters should be sorted by cohesion descending");
        }
    }

    #[rstest]
    fn test_depth1_sorted_by_cohesion(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for window in result.clusters.windows(2) {
            assert!(window[0].cohesion >= window[1].cohesion,
                "Clusters should be sorted by cohesion descending");
        }
    }

    // =========================================================================
    // Module filter tests (boolean filter logic: && vs || matters)
    // =========================================================================

    #[rstest]
    fn test_filter_narrows_modules(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, Some("Controller"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Only Controller module passes the filter, but other namespaces
        // may appear if they have cross-namespace calls with Controller
        assert!(result.clusters.iter().any(|c| c.namespace == "MyApp.Controller"),
            "Controller namespace should be present");
        assert_eq!(result.total_clusters, 1,
            "Only Controller namespace should appear in clusters");
    }

    #[rstest]
    fn test_filter_accounts_incoming_from_filtered_out(populated_db: Box<dyn db::backend::Database>) {
        // When filtering to "Accounts", only Accounts modules pass the filter.
        // Cross-ns calls from non-filtered modules count as incoming for Accounts.
        let cmd = clusters_cmd(2, Some("Accounts"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let accounts = result.clusters.iter().find(|c| c.namespace == "MyApp.Accounts").unwrap();
        // Accounts has incoming from: Controller*2, Service*1, Cache*1 = 4
        assert_eq!(accounts.incoming_calls, 4,
            "Accounts should have 4 incoming calls from outside the filter");
    }

    #[rstest]
    fn test_filter_accounts_outgoing(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, Some("Accounts"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let accounts = result.clusters.iter().find(|c| c.namespace == "MyApp.Accounts").unwrap();
        // Accounts outgoing to: Repo*2, Controller*1 = 3
        assert_eq!(accounts.outgoing_calls, 3,
            "Accounts should have 3 outgoing calls");
    }

    #[rstest]
    fn test_filter_no_match(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, Some("NonExistent"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clusters, 0);
        assert!(result.clusters.is_empty());
    }

    #[rstest]
    fn test_filter_neither_in_filter_skipped(populated_db: Box<dyn db::backend::Database>) {
        // When filtering to "Metrics", calls between e.g. Controller and Accounts
        // (where neither is in the filtered set) should be skipped entirely.
        let cmd = clusters_cmd(2, Some("Metrics"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clusters, 1);
        let metrics = &result.clusters[0];
        assert_eq!(metrics.namespace, "MyApp.Metrics");
        // Metrics: outgoing=1 (to Logger, not in filter), incoming=1 (from Notifier, not in filter)
        assert_eq!(metrics.outgoing_calls, 1);
        assert_eq!(metrics.incoming_calls, 1);
    }

    // =========================================================================
    // Cross-dependencies (show_dependencies flag)
    // =========================================================================

    #[rstest]
    fn test_show_dependencies_false(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.cross_dependencies.is_empty(),
            "Cross dependencies should be empty when show_dependencies is false");
    }

    #[rstest]
    fn test_show_dependencies_true(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, true);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(!result.cross_dependencies.is_empty(),
            "Cross dependencies should be populated when show_dependencies is true");
    }

    #[rstest]
    fn test_cross_deps_from_ne_to(populated_db: Box<dyn db::backend::Database>) {
        // All cross-dependencies should have from_namespace != to_namespace
        let cmd = clusters_cmd(2, None, true);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for dep in &result.cross_dependencies {
            assert_ne!(dep.from_namespace, dep.to_namespace,
                "Cross dep should not be from {} to {}", dep.from_namespace, dep.to_namespace);
        }
    }

    #[rstest]
    fn test_cross_deps_sorted_by_count(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, true);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for window in result.cross_dependencies.windows(2) {
            assert!(window[0].call_count >= window[1].call_count,
                "Cross deps should be sorted by call_count descending");
        }
    }

    #[rstest]
    fn test_cross_deps_controller_to_accounts(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, true);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let ctrl_to_accts = result.cross_dependencies.iter()
            .find(|d| d.from_namespace == "MyApp.Controller" && d.to_namespace == "MyApp.Accounts");
        assert!(ctrl_to_accts.is_some(), "Should have Controller -> Accounts dependency");
        assert_eq!(ctrl_to_accts.unwrap().call_count, 2,
            "Controller -> Accounts should have 2 calls");
    }

    #[rstest]
    fn test_depth1_no_cross_deps(populated_db: Box<dyn db::backend::Database>) {
        // At depth 1, all calls are internal, so no cross-dependencies
        let cmd = clusters_cmd(1, None, true);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert!(result.cross_dependencies.is_empty(),
            "No cross-dependencies at depth 1 where everything is in 'MyApp'");
    }

    // =========================================================================
    // CommandRunner (mod.rs:43) -- tests run() to kill "xyzzy" mutants
    // =========================================================================

    #[rstest]
    fn test_run_returns_non_empty_table(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = clusters_cmd(1, None, false);
        let output = cmd.run(&*populated_db, OutputFormat::Table).expect("run should succeed");

        assert!(!output.is_empty(), "run() should return non-empty output");
        assert!(output.contains("Module Clusters"), "Table output should contain header");
        assert!(output.contains("MyApp"), "Table should contain namespace");
    }

    #[rstest]
    fn test_run_returns_valid_json(populated_db: Box<dyn db::backend::Database>) {
        use crate::commands::CommandRunner;
        use crate::output::OutputFormat;

        let cmd = clusters_cmd(2, None, false);
        let output = cmd.run(&*populated_db, OutputFormat::Json).expect("run should succeed");

        let parsed: serde_json::Value = serde_json::from_str(&output)
            .expect("run() JSON output should be valid JSON");
        assert_eq!(parsed["depth"], 2);
        assert!(parsed["clusters"].is_array());
    }

    // =========================================================================
    // Accumulator precision: ensure += is correct (not -= or *=)
    // =========================================================================

    #[rstest]
    fn test_accumulators_sum_correctly_depth1(populated_db: Box<dyn db::backend::Database>) {
        // At depth 1 all calls are internal. Sum of internal calls across
        // all clusters should equal the total inter-module call count (20).
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let total_internal: i64 = result.clusters.iter().map(|c| c.internal_calls).sum();
        assert_eq!(total_internal, 20, "Sum of internal calls at depth 1 should be 20");
    }

    #[rstest]
    fn test_accumulators_sum_correctly_depth2(populated_db: Box<dyn db::backend::Database>) {
        // At depth 2, all calls are cross-namespace.
        // Each call is counted once as outgoing for the caller and once as incoming for the callee.
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let total_outgoing: i64 = result.clusters.iter().map(|c| c.outgoing_calls).sum();
        let total_incoming: i64 = result.clusters.iter().map(|c| c.incoming_calls).sum();

        assert_eq!(total_outgoing, 20, "Total outgoing calls should equal 20");
        assert_eq!(total_incoming, 20, "Total incoming calls should equal 20");
        assert_eq!(total_outgoing, total_incoming, "Total outgoing should equal total incoming");
    }

    // =========================================================================
    // Cohesion and instability ratio arithmetic
    // =========================================================================

    #[rstest]
    fn test_cohesion_uses_addition_not_subtraction(populated_db: Box<dyn db::backend::Database>) {
        // Cohesion = internal / (internal + outgoing + incoming)
        // At depth 1: 20 / (20 + 0 + 0) = 1.0
        // If + were replaced with -, denominator would be 20 - 0 - 0 = 20 still (degenerate)
        // So also test at depth 2 where we know exact values
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // At depth 2: internal=0 for all, so cohesion = 0 / (outgoing + incoming) = 0.0
        // If / were replaced with %, 0 % anything = 0 (same result)
        // If / were replaced with *, 0 * anything = 0 (same result)
        // But the total_interactions > 0 check matters -- all clusters have outgoing+incoming > 0
        for cluster in &result.clusters {
            assert!(cluster.outgoing_calls + cluster.incoming_calls > 0,
                "All depth-2 clusters should have some external calls");
            assert!((cluster.cohesion - 0.0).abs() < f64::EPSILON,
                "Cohesion should be 0.0 when internal=0, got {} for {}", cluster.cohesion, cluster.namespace);
        }
    }

    #[rstest]
    fn test_instability_uses_division(populated_db: Box<dyn db::backend::Database>) {
        // Repo: outgoing=1, incoming=3, instability = 1/4 = 0.25
        // If / -> %, 1 % 4 = 1 (different!)
        // If / -> *, 1 * 4 = 4 (different!)
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let repo = result.clusters.iter().find(|c| c.namespace == "MyApp.Repo").unwrap();
        assert!((repo.instability - 0.25).abs() < 0.001,
            "Repo instability should be exactly 0.25, got {}", repo.instability);
    }

    #[rstest]
    fn test_cohesion_boundary_total_interactions_threshold(populated_db: Box<dyn db::backend::Database>) {
        // The > 0 check on total_interactions matters:
        // If > were replaced with ==, only clusters with total_interactions == 0 would compute cohesion
        // If > were replaced with <, no clusters would compute cohesion
        // If > were replaced with >=, clusters with 0 would also compute (division by zero)
        let cmd = clusters_cmd(1, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        // total_interactions = 20 > 0, so cohesion should be computed (= 1.0)
        assert!((cluster.cohesion - 1.0).abs() < f64::EPSILON,
            "Cohesion should be 1.0, got {}", cluster.cohesion);
    }

    #[rstest]
    fn test_instability_boundary_external_total_threshold(populated_db: Box<dyn db::backend::Database>) {
        // Similar to cohesion: > 0 check on external_total
        // Controller: external_total = 1 + 5 = 6 > 0
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let controller = result.clusters.iter().find(|c| c.namespace == "MyApp.Controller").unwrap();
        // If > became ==, 6 == 0 is false, instability would be 0.0 (wrong)
        assert!(controller.instability > 0.0,
            "Controller instability should be > 0, got {}", controller.instability);
    }

    // =========================================================================
    // Mixed internal+external: depth=1 with filter creates non-degenerate cohesion
    // Kills mutants on line 130 (+ with - and *)
    // =========================================================================

    #[rstest]
    fn test_depth1_filter_mixed_cohesion(populated_db: Box<dyn db::backend::Database>) {
        // Filter "er" matches Controller, Service, Notifier, Logger.
        // At depth 1, all share namespace "MyApp".
        //
        // Internal calls (both caller and callee in filter, same namespace):
        //   Controller -> Service (1), Controller -> Notifier (1),
        //   Service -> Notifier (1), Service -> Logger (1)
        //   = 4 internal
        //
        // Outgoing (caller in filter, callee NOT, same namespace = line 116):
        //   Controller -> Accounts (2), Controller -> Events (1),
        //   Service -> Accounts (1),
        //   Logger -> Repo (1), Logger -> Events (1), Notifier -> Metrics (1)
        //   = 7 outgoing
        //
        // Incoming: 0 (same-ns calls from non-filtered callers fall through)
        //
        // total_interactions = 4 + 7 + 0 = 11
        // cohesion = 4 / 11 ~= 0.3636
        let cmd = clusters_cmd(1, Some("er"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.total_clusters, 1);
        let cluster = &result.clusters[0];
        assert_eq!(cluster.namespace, "MyApp");
        assert_eq!(cluster.internal_calls, 4, "4 internal calls among filtered modules");
        assert_eq!(cluster.outgoing_calls, 7, "7 outgoing calls to non-filtered modules");
        assert_eq!(cluster.incoming_calls, 0, "No incoming from non-filtered callers in same ns");

        // cohesion = 4 / (4 + 7 + 0) = 4/11 ~= 0.3636
        // This catches + -> - on line 130:47: 4 - 7 + 0 = -3, < 0, cohesion=0.0 (wrong!)
        // This catches + -> * on line 130:47: 4 * 7 + 0 = 28, cohesion=4/28=0.143 (wrong!)
        // This catches + -> * on line 130:58: (4 + 7) * 0 = 0, cohesion=0.0 (wrong!)
        let expected_cohesion = 4.0 / 11.0;
        assert!((cluster.cohesion - expected_cohesion).abs() < 0.01,
            "Cohesion should be ~{:.4}, got {}", expected_cohesion, cluster.cohesion);
    }

    #[rstest]
    fn test_depth1_filter_mixed_instability(populated_db: Box<dyn db::backend::Database>) {
        // Same filter "er" at depth 1:
        // external_total = incoming + outgoing = 0 + 7 = 7
        // instability = outgoing / external_total = 7 / 7 = 1.0
        let cmd = clusters_cmd(1, Some("er"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        // Kills + -> - on line 139: 0 - 7 = -7, < 0, instability=0.0 (wrong!)
        // Kills + -> * on line 139: 0 * 7 = 0, instability=0.0 or div-by-zero (wrong!)
        assert!((cluster.instability - 1.0).abs() < 0.01,
            "Instability should be 1.0, got {}", cluster.instability);
    }

    #[rstest]
    fn test_depth1_filter_mixed_cohesion_is_not_zero(populated_db: Box<dyn db::backend::Database>) {
        // Explicitly test that cohesion > 0 to catch threshold mutants
        let cmd = clusters_cmd(1, Some("er"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let cluster = &result.clusters[0];
        assert!(cluster.cohesion > 0.0,
            "Cohesion should be > 0 when there are internal calls, got {}", cluster.cohesion);
        assert!(cluster.cohesion < 1.0,
            "Cohesion should be < 1 when there are also external calls, got {}", cluster.cohesion);
    }

    #[rstest]
    fn test_depth2_filter_with_incoming(populated_db: Box<dyn db::backend::Database>) {
        // At depth 2 with filter "er", some clusters have incoming > 0:
        // MyApp.Controller: outgoing=5, incoming=1 (Accounts->Controller)
        //
        // For instability: external_total = incoming + outgoing = 1 + 5 = 6
        // instability = 5 / 6 ~= 0.833
        // With + -> - on 139: 1 - 5 = -4, < 0, instability = 0.0 (different!)
        let cmd = clusters_cmd(2, Some("er"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        let controller = result.clusters.iter()
            .find(|c| c.namespace == "MyApp.Controller").unwrap();
        assert!(controller.incoming_calls > 0, "Controller should have incoming calls");
        assert!(controller.outgoing_calls > 0, "Controller should have outgoing calls");

        let expected_instability = controller.outgoing_calls as f64
            / (controller.incoming_calls + controller.outgoing_calls) as f64;
        assert!((controller.instability - expected_instability).abs() < 0.01,
            "Instability formula should use + not -, expected {}, got {}",
            expected_instability, controller.instability);
    }

    // =========================================================================
    // Custom fixture with 3-level modules to test cohesion with internal > 0
    // AND incoming > 0 simultaneously. This kills line 130:58 (+ with - on incoming).
    // =========================================================================

    /// Creates a custom SurrealDB with 3-level module names to produce
    /// clusters with both internal calls and incoming calls at depth 2.
    ///
    /// Modules:
    ///   App.Core.User, App.Core.Admin (namespace "App.Core" at depth 2)
    ///   App.Web.Controller (namespace "App.Web" at depth 2)
    ///
    /// Calls:
    ///   App.Core.User -> App.Core.Admin (internal to App.Core)
    ///   App.Web.Controller -> App.Core.User (cross-ns: incoming to App.Core)
    ///   App.Core.Admin -> App.Web.Controller (cross-ns: outgoing from App.Core)
    ///
    /// At depth 2:
    ///   App.Core: internal=1, outgoing=1, incoming=1
    ///   App.Web: internal=0, outgoing=1, incoming=1
    fn three_level_db() -> Box<dyn db::backend::Database> {
        let db = db::backend::open_mem_database().expect("open in-memory db");
        db::queries::schema::create_schema(&*db).expect("create schema");

        // Create modules
        db.execute_query_no_params(
            r#"CREATE modules:["App.Core.User"] SET name = "App.Core.User", file = "", source = "unknown""#
        ).expect("create App.Core.User module");
        db.execute_query_no_params(
            r#"CREATE modules:["App.Core.Admin"] SET name = "App.Core.Admin", file = "", source = "unknown""#
        ).expect("create App.Core.Admin module");
        db.execute_query_no_params(
            r#"CREATE modules:["App.Web.Controller"] SET name = "App.Web.Controller", file = "", source = "unknown""#
        ).expect("create App.Web.Controller module");

        // Create functions
        db.execute_query_no_params(
            r#"CREATE functions:["App.Core.User", "get", 1] SET module_name = "App.Core.User", name = "get", arity = 1, kind = "def", file = "user.ex", start_line = 1"#
        ).expect("create user.get");
        db.execute_query_no_params(
            r#"CREATE functions:["App.Core.Admin", "promote", 1] SET module_name = "App.Core.Admin", name = "promote", arity = 1, kind = "def", file = "admin.ex", start_line = 1"#
        ).expect("create admin.promote");
        db.execute_query_no_params(
            r#"CREATE functions:["App.Web.Controller", "index", 2] SET module_name = "App.Web.Controller", name = "index", arity = 2, kind = "def", file = "controller.ex", start_line = 1"#
        ).expect("create controller.index");

        // Create calls:
        // 1. User -> Admin (internal to App.Core at depth 2)
        db.execute_query_no_params(r#"
            RELATE functions:["App.Core.User", "get", 1]
                ->calls->
                functions:["App.Core.Admin", "promote", 1]
            SET call_type = "remote", caller_kind = "def", file = "user.ex", line = 5
        "#).expect("create call: User -> Admin");

        // 2. Controller -> User (cross-ns: App.Web -> App.Core = incoming for App.Core)
        db.execute_query_no_params(r#"
            RELATE functions:["App.Web.Controller", "index", 2]
                ->calls->
                functions:["App.Core.User", "get", 1]
            SET call_type = "remote", caller_kind = "def", file = "controller.ex", line = 3
        "#).expect("create call: Controller -> User");

        // 3. Admin -> Controller (cross-ns: App.Core -> App.Web = outgoing for App.Core)
        db.execute_query_no_params(r#"
            RELATE functions:["App.Core.Admin", "promote", 1]
                ->calls->
                functions:["App.Web.Controller", "index", 2]
            SET call_type = "remote", caller_kind = "def", file = "admin.ex", line = 5
        "#).expect("create call: Admin -> Controller");

        db
    }

    #[test]
    fn test_three_level_modules_cohesion_with_incoming(
    ) {
        let db = three_level_db();
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*db).expect("Execute should succeed");

        assert_eq!(result.total_clusters, 2);

        let core = result.clusters.iter()
            .find(|c| c.namespace == "App.Core").unwrap();

        // App.Core: internal=1 (User->Admin), outgoing=1 (Admin->Controller), incoming=1 (Controller->User)
        assert_eq!(core.internal_calls, 1);
        assert_eq!(core.outgoing_calls, 1);
        assert_eq!(core.incoming_calls, 1);

        // total_interactions = 1 + 1 + 1 = 3
        // cohesion = 1 / 3 ~= 0.333
        //
        // With + -> - on line 130:58 (+ incoming -> - incoming):
        //   total = 1 + 1 - 1 = 1, cohesion = 1/1 = 1.0 (DIFFERENT!)
        // With + -> * on line 130:58 (+ incoming -> * incoming):
        //   total = 1 + 1 * 1 = 2, cohesion = 1/2 = 0.5 (DIFFERENT!)
        let expected_cohesion = 1.0 / 3.0;
        assert!((core.cohesion - expected_cohesion).abs() < 0.01,
            "App.Core cohesion should be ~{:.4}, got {:.4}", expected_cohesion, core.cohesion);
    }

    #[test]
    fn test_three_level_modules_instability(
    ) {
        let db = three_level_db();
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*db).expect("Execute should succeed");

        let core = result.clusters.iter()
            .find(|c| c.namespace == "App.Core").unwrap();

        // instability = outgoing / (incoming + outgoing) = 1 / (1 + 1) = 0.5
        assert!((core.instability - 0.5).abs() < 0.01,
            "App.Core instability should be 0.5, got {}", core.instability);

        let web = result.clusters.iter()
            .find(|c| c.namespace == "App.Web").unwrap();

        // App.Web: internal=0, outgoing=1 (Controller->User), incoming=1 (Admin->Controller)
        assert_eq!(web.internal_calls, 0);
        assert_eq!(web.outgoing_calls, 1);
        assert_eq!(web.incoming_calls, 1);
        // instability = 1 / (1 + 1) = 0.5
        assert!((web.instability - 0.5).abs() < 0.01,
            "App.Web instability should be 0.5, got {}", web.instability);
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[rstest]
    fn test_depth3_same_as_depth2_for_two_level_modules(populated_db: Box<dyn db::backend::Database>) {
        // Modules are 2-level (e.g., "MyApp.Controller"). Depth 3 gives same namespaces as depth 2.
        let cmd2 = clusters_cmd(2, None, false);
        let result2 = cmd2.execute(&*populated_db).expect("Execute should succeed");

        let cmd3 = clusters_cmd(3, None, false);
        let result3 = cmd3.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result2.total_clusters, result3.total_clusters);
    }

    #[rstest]
    fn test_each_cluster_has_at_least_one_module(populated_db: Box<dyn db::backend::Database>) {
        let cmd = clusters_cmd(2, None, false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        for cluster in &result.clusters {
            assert!(cluster.module_count >= 1,
                "Cluster {} should have at least 1 module", cluster.namespace);
        }
    }

    #[rstest]
    fn test_filter_same_ns_callee_outside_filter(populated_db: Box<dyn db::backend::Database>) {
        // Test the branch at line 116: caller_in_filter && !callee_in_filter && same namespace
        // With the current 2-level modules, all modules in same namespace are the same module.
        // The self-calls are excluded by the query. This branch handles the edge case where
        // caller is in filter but callee in the same namespace is not.
        // We can exercise this with a module filter that matches some but not all within a namespace.
        // With 2-level modules at depth 1, all modules share "MyApp" namespace.
        // Filter to "Controller" -- only MyApp.Controller is in filter.
        // At depth 1, all share namespace "MyApp". Calls from Controller to others
        // are same-namespace but callee is not in filter -- should be counted as outgoing.
        let cmd = clusters_cmd(1, Some("Controller"), false);
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // Only "MyApp" namespace should appear (Controller is in it)
        assert_eq!(result.total_clusters, 1);
        let cluster = &result.clusters[0];
        assert_eq!(cluster.namespace, "MyApp");

        // Controller makes 5 outgoing cross-module calls, none of the callees
        // pass the "Controller" filter (they're Accounts, Service, etc.)
        // Same namespace (MyApp at depth 1) but callee not in filter -> counted as outgoing
        assert_eq!(cluster.outgoing_calls, 5,
            "Calls from Controller to non-Controller modules in same ns should be outgoing");
        // Incoming: Accounts.notify_change -> Controller.handle_event
        // Accounts is not in filter, Controller is. Same namespace. But this hits
        // the caller_in_filter=false callee_in_filter=true case, which just continues (line 93)
        // Actually no -- line 93 skips when NEITHER is in filter. If callee is in filter,
        // it doesn't skip. Then line 97: same ns, both in filter? No (caller not in filter).
        // Line 100: different ns? No, same ns at depth 1.
        // Line 116: caller_in_filter && !callee_in_filter? caller=Accounts not in filter. No.
        // So the call from Accounts->Controller is... not counted? Let's check:
        // caller=Accounts (not in filter), callee=Controller (in filter), same ns (MyApp)
        // Line 93: !false && !true = false, so doesn't skip
        // Line 97: same_ns && true && false = false
        // Line 100: caller_ns != callee_ns? No (both MyApp)
        // Line 116: false && !true = false
        // Falls through without counting! This means incoming = 0 for this case.
        assert_eq!(cluster.incoming_calls, 0,
            "Calls from non-filtered callers in same ns are not counted as incoming");
    }
}
