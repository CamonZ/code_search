use std::collections::{HashMap, HashSet};
use std::error::Error;

use serde::Serialize;

use super::ClustersCmd;
use crate::commands::Execute;
use db::queries::clusters::get_module_calls;

/// A single namespace cluster
#[derive(Debug, Clone, Serialize)]
pub struct ClusterInfo {
    pub namespace: String,
    pub module_count: usize,
    pub internal_calls: i64,
    pub outgoing_calls: i64,
    pub incoming_calls: i64,
    /// Cohesion: internal / (internal + outgoing + incoming)
    /// Range 0-1, higher = more self-contained
    pub cohesion: f64,
    /// Instability: outgoing / (incoming + outgoing)
    /// Range 0-1, 0 = stable (depended upon), 1 = unstable (depends on others)
    pub instability: f64,
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

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
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
                .or_default()
                .insert(module.clone());
        }

        // Count internal, outgoing, and incoming calls per namespace
        let mut internal_calls: HashMap<String, i64> = HashMap::new();
        let mut outgoing_calls: HashMap<String, i64> = HashMap::new();
        let mut incoming_calls: HashMap<String, i64> = HashMap::new();
        let mut cross_deps: HashMap<(String, String), i64> = HashMap::new();

        for call in calls {
            let caller_ns = extract_namespace(&call.caller_module, self.depth);
            let callee_ns = extract_namespace(&call.callee_module, self.depth);

            let caller_in_filter = filtered_modules.contains(&call.caller_module);
            let callee_in_filter = filtered_modules.contains(&call.callee_module);

            // Skip calls where neither module is in our filtered set
            if !caller_in_filter && !callee_in_filter {
                continue;
            }

            if caller_ns == callee_ns && caller_in_filter && callee_in_filter {
                // Internal call (same namespace, both in filter)
                *internal_calls.entry(caller_ns).or_insert(0) += 1;
            } else if caller_ns != callee_ns {
                // Cross-namespace call
                // Count as outgoing for caller namespace (if in filter)
                if caller_in_filter {
                    *outgoing_calls.entry(caller_ns.clone()).or_insert(0) += 1;
                }
                // Count as incoming for callee namespace (if in filter)
                if callee_in_filter {
                    *incoming_calls.entry(callee_ns.clone()).or_insert(0) += 1;
                }

                // Track cross-dependencies (from caller's perspective)
                if caller_in_filter {
                    let key = (caller_ns, callee_ns);
                    *cross_deps.entry(key).or_insert(0) += 1;
                }
            } else if caller_in_filter && !callee_in_filter {
                // Same namespace but callee outside filter - count as outgoing
                *outgoing_calls.entry(caller_ns.clone()).or_insert(0) += 1;
            }
        }

        // Build cluster info
        let mut clusters = Vec::new();
        for (namespace, modules) in namespace_modules {
            let internal = internal_calls.get(&namespace).copied().unwrap_or(0);
            let outgoing = outgoing_calls.get(&namespace).copied().unwrap_or(0);
            let incoming = incoming_calls.get(&namespace).copied().unwrap_or(0);

            // Cohesion: internal / (internal + outgoing + incoming)
            let total_interactions = internal + outgoing + incoming;
            let cohesion = if total_interactions > 0 {
                internal as f64 / total_interactions as f64
            } else {
                0.0
            };

            // Instability: outgoing / (incoming + outgoing)
            // 0 = stable (depended upon), 1 = unstable (depends on others)
            let external_total = incoming + outgoing;
            let instability = if external_total > 0 {
                outgoing as f64 / external_total as f64
            } else {
                0.0
            };

            clusters.push(ClusterInfo {
                namespace,
                module_count: modules.len(),
                internal_calls: internal,
                outgoing_calls: outgoing,
                incoming_calls: incoming,
                cohesion,
                instability,
            });
        }

        // Sort by cohesion descending, then by internal calls
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
        let outgoing = 0;
        let incoming = 0;
        let total = internal + outgoing + incoming;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert_eq!(cohesion, 1.0);
    }

    #[test]
    fn test_cohesion_calculation_all_external() {
        // If all calls are external (outgoing + incoming), cohesion should be 0.0
        let internal = 0;
        let outgoing = 5;
        let incoming = 5;
        let total = internal + outgoing + incoming;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert_eq!(cohesion, 0.0);
    }

    #[test]
    fn test_cohesion_calculation_mixed() {
        // Mixed: internal=45, outgoing=8, incoming=4 → 45/(45+8+4) = 45/57 ≈ 0.79
        let internal = 45;
        let outgoing = 8;
        let incoming = 4;
        let total = internal + outgoing + incoming;
        let cohesion = if total > 0 {
            internal as f64 / total as f64
        } else {
            0.0
        };
        assert!((cohesion - 0.79).abs() < 0.01);
    }

    #[test]
    fn test_instability_calculation() {
        // Instability = outgoing / (incoming + outgoing)
        // outgoing=8, incoming=4 → 8/12 ≈ 0.67 (unstable, depends on others)
        let outgoing = 8;
        let incoming = 4;
        let external_total = incoming + outgoing;
        let instability = if external_total > 0 {
            outgoing as f64 / external_total as f64
        } else {
            0.0
        };
        assert!((instability - 0.67).abs() < 0.01);
    }

    #[test]
    fn test_instability_stable_namespace() {
        // A namespace with only incoming calls is stable (instability = 0)
        let outgoing = 0;
        let incoming = 10;
        let external_total = incoming + outgoing;
        let instability = if external_total > 0 {
            outgoing as f64 / external_total as f64
        } else {
            0.0
        };
        assert_eq!(instability, 0.0);
    }

    #[test]
    fn test_instability_unstable_namespace() {
        // A namespace with only outgoing calls is unstable (instability = 1)
        let outgoing = 10;
        let incoming = 0;
        let external_total = incoming + outgoing;
        let instability = if external_total > 0 {
            outgoing as f64 / external_total as f64
        } else {
            0.0
        };
        assert_eq!(instability, 1.0);
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
            outgoing_calls: 8,
            incoming_calls: 4,
            cohesion: 0.79,
            instability: 0.67,
        };

        assert_eq!(cluster.namespace, "MyApp.Accounts");
        assert_eq!(cluster.module_count, 5);
        assert_eq!(cluster.internal_calls, 45);
        assert_eq!(cluster.outgoing_calls, 8);
        assert_eq!(cluster.incoming_calls, 4);
        assert!((cluster.cohesion - 0.79).abs() < 0.001);
        assert!((cluster.instability - 0.67).abs() < 0.001);
    }
}
