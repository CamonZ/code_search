//! Output formatting for clusters command results.

use super::execute::ClustersResult;
use crate::output::Outputable;

impl Outputable for ClustersResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push("Module Clusters (by namespace)".to_string());
        lines.push(String::new());

        if self.clusters.is_empty() {
            lines.push("No clusters found.".to_string());
            return lines.join("\n");
        }

        // Summary
        lines.push(format!("Found {} cluster(s):", self.total_clusters));
        lines.push(String::new());

        // Table header
        let header = format!(
            "{:<30} {:>8} {:>10} {:>10} {:>10}",
            "Cluster", "Modules", "Internal", "External", "Cohesion"
        );
        lines.push(header);
        lines.push("-".repeat(79));

        // Table rows
        for cluster in &self.clusters {
            let cohesion_str = format!("{:.2}", cluster.cohesion);
            let row = format!(
                "{:<30} {:>8} {:>10} {:>10} {:>10}",
                cluster.namespace,
                cluster.module_count,
                cluster.internal_calls,
                cluster.external_calls,
                cohesion_str
            );
            lines.push(row);
        }

        // Cross-dependencies section
        if !self.cross_dependencies.is_empty() {
            lines.push(String::new());
            lines.push("Cross-Namespace Dependencies:".to_string());
            lines.push(String::new());

            for dep in &self.cross_dependencies {
                lines.push(format!(
                    "  {} → {}: {} calls",
                    dep.from_namespace, dep.to_namespace, dep.call_count
                ));
            }
        }

        lines.join("\n")
    }
}
