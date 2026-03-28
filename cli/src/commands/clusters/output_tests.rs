//! Output formatting tests for clusters command.

#[cfg(test)]
mod tests {
    use super::super::execute::{ClusterInfo, CrossDependency, ClustersResult};
    use crate::output::{OutputFormat, Outputable};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Module Clusters (depth: 2)

No clusters found.";

    const SINGLE_TABLE: &str = "\
Module Clusters (depth: 1)

Found 1 cluster(s):

Cluster Modules Internal   Out    In Cohesion Instab
----------------------------------------------------
MyApp         9       20     0     0     1.00   0.00";

    const MULTIPLE_TABLE: &str = "\
Module Clusters (depth: 2)

Found 3 cluster(s):

Cluster          Modules Internal   Out    In Cohesion Instab
-------------------------------------------------------------
MyApp.Accounts         5       45     8     4     0.79   0.67
MyApp.Controller       3        0     5     1     0.00   0.83
MyApp.Repo             4        0     1     3     0.00   0.25";

    const WITH_DEPS_TABLE: &str = "\
Module Clusters (depth: 2)

Found 2 cluster(s):

Cluster          Modules Internal   Out    In Cohesion Instab
-------------------------------------------------------------
MyApp.Accounts         5       10     3     2     0.67   0.60
MyApp.Controller       3        0     5     1     0.00   0.83

Cross-Namespace Dependencies:

  MyApp.Controller \u{2192} MyApp.Accounts: 3 calls
  MyApp.Accounts \u{2192} MyApp.Controller: 1 calls";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ClustersResult {
        ClustersResult {
            depth: 2,
            total_clusters: 0,
            clusters: vec![],
            cross_dependencies: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ClustersResult {
        ClustersResult {
            depth: 1,
            total_clusters: 1,
            clusters: vec![ClusterInfo {
                namespace: "MyApp".to_string(),
                module_count: 9,
                internal_calls: 20,
                outgoing_calls: 0,
                incoming_calls: 0,
                cohesion: 1.0,
                instability: 0.0,
            }],
            cross_dependencies: vec![],
        }
    }

    #[fixture]
    fn multiple_result() -> ClustersResult {
        ClustersResult {
            depth: 2,
            total_clusters: 3,
            clusters: vec![
                ClusterInfo {
                    namespace: "MyApp.Accounts".to_string(),
                    module_count: 5,
                    internal_calls: 45,
                    outgoing_calls: 8,
                    incoming_calls: 4,
                    cohesion: 45.0 / 57.0,
                    instability: 8.0 / 12.0,
                },
                ClusterInfo {
                    namespace: "MyApp.Controller".to_string(),
                    module_count: 3,
                    internal_calls: 0,
                    outgoing_calls: 5,
                    incoming_calls: 1,
                    cohesion: 0.0,
                    instability: 5.0 / 6.0,
                },
                ClusterInfo {
                    namespace: "MyApp.Repo".to_string(),
                    module_count: 4,
                    internal_calls: 0,
                    outgoing_calls: 1,
                    incoming_calls: 3,
                    cohesion: 0.0,
                    instability: 0.25,
                },
            ],
            cross_dependencies: vec![],
        }
    }

    #[fixture]
    fn with_deps_result() -> ClustersResult {
        ClustersResult {
            depth: 2,
            total_clusters: 2,
            clusters: vec![
                ClusterInfo {
                    namespace: "MyApp.Accounts".to_string(),
                    module_count: 5,
                    internal_calls: 10,
                    outgoing_calls: 3,
                    incoming_calls: 2,
                    cohesion: 10.0 / 15.0,
                    instability: 3.0 / 5.0,
                },
                ClusterInfo {
                    namespace: "MyApp.Controller".to_string(),
                    module_count: 3,
                    internal_calls: 0,
                    outgoing_calls: 5,
                    incoming_calls: 1,
                    cohesion: 0.0,
                    instability: 5.0 / 6.0,
                },
            ],
            cross_dependencies: vec![
                CrossDependency {
                    from_namespace: "MyApp.Controller".to_string(),
                    to_namespace: "MyApp.Accounts".to_string(),
                    call_count: 3,
                },
                CrossDependency {
                    from_namespace: "MyApp.Accounts".to_string(),
                    to_namespace: "MyApp.Controller".to_string(),
                    call_count: 1,
                },
            ],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    #[rstest]
    fn test_to_table_empty(empty_result: ClustersResult) {
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_to_table_single(single_result: ClustersResult) {
        let output = single_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: ClustersResult) {
        let output = multiple_result.to_table();
        assert_eq!(output, MULTIPLE_TABLE);
    }

    #[rstest]
    fn test_to_table_with_deps(with_deps_result: ClustersResult) {
        let output = with_deps_result.to_table();
        assert_eq!(output, WITH_DEPS_TABLE);
    }

    #[rstest]
    fn test_to_table_not_empty_string(single_result: ClustersResult) {
        // Kills mutant: to_table -> String::new()
        let output = single_result.to_table();
        assert!(!output.is_empty(), "to_table should never return empty string for non-empty result");
    }

    #[rstest]
    fn test_to_table_not_xyzzy(single_result: ClustersResult) {
        // Kills mutant: to_table -> "xyzzy"
        let output = single_result.to_table();
        assert_ne!(output, "xyzzy");
        assert!(output.contains("Module Clusters"), "Should contain proper header");
    }

    // =========================================================================
    // Verify the dynamic column width (namespace_width + 45 for separator)
    // =========================================================================

    #[rstest]
    fn test_separator_width_matches_namespace(multiple_result: ClustersResult) {
        let output = multiple_result.to_table();
        let lines: Vec<&str> = output.lines().collect();
        // Lines: [0] header, [1] blank, [2] summary, [3] blank, [4] table_header, [5] separator
        let separator = lines[5];
        // max namespace len is "MyApp.Controller".len() = 16
        // namespace_width = max(16, 7) = 16
        // separator = 16 + 45 = 61 dashes
        assert_eq!(separator.len(), 61, "Separator should be namespace_width(16) + 45 = 61 chars");
        assert!(separator.chars().all(|c| c == '-'), "Separator should be all dashes");
    }

    // =========================================================================
    // Cross-dependencies section in output
    // =========================================================================

    #[rstest]
    fn test_cross_deps_not_shown_when_empty(single_result: ClustersResult) {
        let output = single_result.to_table();
        assert!(!output.contains("Cross-Namespace Dependencies"),
            "Should not show cross-deps section when empty");
    }

    #[rstest]
    fn test_cross_deps_shown_when_present(with_deps_result: ClustersResult) {
        let output = with_deps_result.to_table();
        assert!(output.contains("Cross-Namespace Dependencies:"),
            "Should show cross-deps section header");
        assert!(output.contains("MyApp.Controller"), "Should contain from namespace");
        assert!(output.contains("MyApp.Accounts"), "Should contain to namespace");
        assert!(output.contains("3 calls"), "Should contain call count");
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_format_json(single_result: ClustersResult) {
        let output = single_result.format(OutputFormat::Json);
        assert!(output.contains("\"depth\": 1"));
        assert!(output.contains("\"total_clusters\": 1"));
        assert!(output.contains("\"clusters\""));
        assert!(output.contains("\"namespace\": \"MyApp\""));
        assert!(output.contains("\"module_count\": 9"));
        assert!(output.contains("\"internal_calls\": 20"));
        assert!(output.contains("\"cohesion\": 1.0"));
    }

    #[rstest]
    fn test_format_json_empty(empty_result: ClustersResult) {
        let output = empty_result.format(OutputFormat::Json);
        assert!(output.contains("\"depth\": 2"));
        assert!(output.contains("\"total_clusters\": 0"));
        assert!(output.contains("\"clusters\": []"));
    }

    #[rstest]
    fn test_format_json_no_cross_deps_when_empty(single_result: ClustersResult) {
        let output = single_result.format(OutputFormat::Json);
        // cross_dependencies uses skip_serializing_if = "Vec::is_empty"
        assert!(!output.contains("cross_dependencies"),
            "Empty cross_dependencies should be skipped in JSON");
    }

    #[rstest]
    fn test_format_json_includes_cross_deps(with_deps_result: ClustersResult) {
        let output = with_deps_result.format(OutputFormat::Json);
        assert!(output.contains("\"cross_dependencies\""),
            "Non-empty cross_dependencies should be in JSON");
        assert!(output.contains("\"call_count\": 3"));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_format_toon(single_result: ClustersResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("depth"));
        assert!(output.contains("total_clusters"));
        assert!(output.contains("clusters"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: ClustersResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("depth"));
        assert!(output.contains("clusters"));
    }
}
