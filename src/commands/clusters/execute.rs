use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde::Serialize;

use super::ClustersCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::clusters::get_module_calls;

/// A single namespace cluster
#[derive(Debug, Clone, Serialize)]
pub struct ClusterInfo {
    pub namespace: String,
    pub module_count: usize,
    pub internal_calls: i64,
    pub external_calls: i64,
    pub cohesion: f64,
}

/// A cross-namespace dependency edge
#[derive(Debug, Clone, Serialize)]
pub struct CrossDependency {
    pub from_namespace: String,
    pub to_namespace: String,
    pub call_count: i64,
}

/// Result of clusters analysis
#[derive(Debug, Serialize)]
pub struct ClustersResult {
    pub depth: usize,
    pub total_clusters: usize,
    pub clusters: Vec<ClusterInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cross_dependencies: Vec<CrossDependency>,
}

impl Execute for ClustersCmd {
    type Output = ClustersResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        // Get all inter-module calls
        let calls = get_module_calls(db, &self.common.project)?;

        // Extract namespace for each module and collect all unique modules
        let mut all_modules = HashSet::new();
        for call in &calls {
            all_modules.insert(call.caller_module.clone());
            all_modules.insert(call.callee_module.clone());
        }

        // Apply module filter if specified (simple substring matching for now)
        // Complex regex filtering happens at query level in other commands
        let filtered_modules: HashSet<String> = if let Some(ref pattern) = self.module {
            all_modules
                .into_iter()
                .filter(|m| m.contains(pattern))
                .collect()
        } else {
            all_modules
        };

        // Build namespace -> modules mapping
        let mut namespace_modules: HashMap<String, HashSet<String>> = HashMap::new();
        for module in &filtered_modules {
            let namespace = extract_namespace(module, self.depth);
            namespace_modules
                .entry(namespace)
                .or_insert_with(HashSet::new)
                .insert(module.clone());
        }

        // Count internal and external calls per namespace
        let mut internal_calls: HashMap<String, i64> = HashMap::new();
        let mut external_calls: HashMap<String, i64> = HashMap::new();
        let mut cross_deps: HashMap<(String, String), i64> = HashMap::new();

        for call in calls {
            let caller_ns = extract_namespace(&call.caller_module, self.depth);
            let callee_ns = extract_namespace(&call.callee_module, self.depth);

            // Only count if both modules are in our filtered set
            if filtered_modules.contains(&call.caller_module) && filtered_modules.contains(&call.callee_module) {
                if caller_ns == callee_ns {
                    // Internal call
                    *internal_calls.entry(caller_ns).or_insert(0) += 1;
                } else {
                    // External call
                    *external_calls.entry(caller_ns.clone()).or_insert(0) += 1;
                    *external_calls.entry(callee_ns.clone()).or_insert(0) += 1;

                    // Track cross-dependencies
                    let key = (caller_ns, callee_ns);
                    *cross_deps.entry(key).or_insert(0) += 1;
                }
            }
        }

        // Build cluster info
        let mut clusters = Vec::new();
        for (namespace, modules) in namespace_modules {
            let internal = internal_calls.get(&namespace).copied().unwrap_or(0);
            let external = external_calls.get(&namespace).copied().unwrap_or(0);
            let total = internal + external;
            let cohesion = if total > 0 {
                internal as f64 / total as f64
            } else {
                0.0
            };

            clusters.push(ClusterInfo {
                namespace,
                module_count: modules.len(),
                internal_calls: internal,
                external_calls: external,
                cohesion,
            });
        }

        // Sort by cohesion descending
        clusters.sort_by(|a, b| {
            b.cohesion
                .partial_cmp(&a.cohesion)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.internal_calls.cmp(&a.internal_calls))
        });

        // Build cross-dependencies if requested
        let cross_dependencies = if self.show_dependencies {
            let mut deps = Vec::new();
            for ((from_ns, to_ns), count) in cross_deps {
                if from_ns != to_ns {
                    deps.push(CrossDependency {
                        from_namespace: from_ns,
                        to_namespace: to_ns,
                        call_count: count,
                    });
                }
            }
            // Sort by call_count descending
            deps.sort_by(|a, b| b.call_count.cmp(&a.call_count));
            deps
        } else {
            Vec::new()
        };

        let total_clusters = clusters.len();

        Ok(ClustersResult {
            depth: self.depth,
            total_clusters,
            clusters,
            cross_dependencies,
        })
    }
}

/// Extract namespace from a module name at the specified depth
///
/// Example: "MyApp.Accounts.Users.Admin" at depth 2 becomes "MyApp.Accounts"
fn extract_namespace(module: &str, depth: usize) -> String {
    module
        .split('.')
        .take(depth)
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace_depth_2() {
        assert_eq!(extract_namespace("MyApp.Accounts.Users", 2), "MyApp.Accounts");
    }

    #[test]
    fn test_extract_namespace_depth_1() {
        assert_eq!(extract_namespace("MyApp.Accounts.Users", 1), "MyApp");
    }

    #[test]
    fn test_extract_namespace_depth_3() {
        assert_eq!(extract_namespace("MyApp.Accounts.Users", 3), "MyApp.Accounts.Users");
    }

    #[test]
    fn test_extract_namespace_single_level() {
        assert_eq!(extract_namespace("MyApp", 2), "MyApp");
    }

    #[test]
    fn test_cohesion_calculation_all_internal() {
        // If all calls are internal, cohesion should be 1.0
        let internal = 10;
        let external = 0;
        let total = internal + external;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert_eq!(cohesion, 1.0);
    }

    #[test]
    fn test_cohesion_calculation_all_external() {
        // If all calls are external, cohesion should be 0.0
        let internal = 0;
        let external = 10;
        let total = internal + external;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert_eq!(cohesion, 0.0);
    }

    #[test]
    fn test_cohesion_calculation_mixed() {
        // Mixed internal/external: 45/(45+12) ≈ 0.79
        let internal = 45;
        let external = 12;
        let total = internal + external;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert!((cohesion - 0.79).abs() < 0.01);
    }

    #[test]
    fn test_clusters_cmd_structure() {
        // Test that ClustersCmd is created correctly with defaults
        let cmd = ClustersCmd {
            depth: 2,
            show_dependencies: false,
            module: None,
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 100,
            },
        };

        assert_eq!(cmd.depth, 2);
        assert!(!cmd.show_dependencies);
        assert_eq!(cmd.module, None);
        assert_eq!(cmd.common.project, "default");
    }

    #[test]
    fn test_clusters_cmd_with_options() {
        let cmd = ClustersCmd {
            depth: 3,
            show_dependencies: true,
            module: Some("MyApp.Core".to_string()),
            common: crate::commands::CommonArgs {
                project: "custom".to_string(),
                regex: false,
                limit: 50,
            },
        };

        assert_eq!(cmd.depth, 3);
        assert!(cmd.show_dependencies);
        assert_eq!(cmd.module, Some("MyApp.Core".to_string()));
        assert_eq!(cmd.common.project, "custom");
    }

    #[test]
    fn test_cross_dependency_structure() {
        let dep = CrossDependency {
            from_namespace: "MyApp.Accounts".to_string(),
            to_namespace: "MyApp.Repo".to_string(),
            call_count: 23,
        };

        assert_eq!(dep.from_namespace, "MyApp.Accounts");
        assert_eq!(dep.to_namespace, "MyApp.Repo");
        assert_eq!(dep.call_count, 23);
    }

    #[test]
    fn test_cluster_info_structure() {
        let cluster = ClusterInfo {
            namespace: "MyApp.Accounts".to_string(),
            module_count: 5,
            internal_calls: 45,
            external_calls: 12,
            cohesion: 0.79,
        };

        assert_eq!(cluster.namespace, "MyApp.Accounts");
        assert_eq!(cluster.module_count, 5);
        assert_eq!(cluster.internal_calls, 45);
        assert_eq!(cluster.external_calls, 12);
        assert!((cluster.cohesion - 0.79).abs() < 0.001);
    }
}
