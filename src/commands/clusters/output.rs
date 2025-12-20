//! Output formatting for clusters command results.

use super::execute::ClustersResult;
use crate::output::Outputable;

impl Outputable for ClustersResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Header with depth
        lines.push(format!("Module Clusters (depth: {})", self.depth));
        lines.push(String::new());

        if self.clusters.is_empty() {
            lines.push("No clusters found.".to_string());
            return lines.join("\n");
        }

        // Summary
        lines.push(format!("Found {} cluster(s):", self.total_clusters));
        lines.push(String::new());

        // Calculate dynamic column width for namespace
        let min_width = 7; // "Cluster".len()
        let max_namespace_len = self
            .clusters
            .iter()
            .map(|c| c.namespace.len())
            .max()
            .unwrap_or(min_width);
        let namespace_width = max_namespace_len.max(min_width);

        // Table header
        // Columns: Cluster(dynamic) Modules(7) Internal(8) Out(5) In(5) Cohesion(8) Instab(6)
        let header = format!(
            "{:<width$} {:>7} {:>8} {:>5} {:>5} {:>8} {:>6}",
            "Cluster",
            "Modules",
            "Internal",
            "Out",
            "In",
            "Cohesion",
            "Instab",
            width = namespace_width
        );
        lines.push(header);
        lines.push("-".repeat(namespace_width + 45));

        // Table rows
        for cluster in &self.clusters {
            let row = format!(
                "{:<width$} {:>7} {:>8} {:>5} {:>5} {:>8.2} {:>6.2}",
                cluster.namespace,
                cluster.module_count,
                cluster.internal_calls,
                cluster.outgoing_calls,
                cluster.incoming_calls,
                cluster.cohesion,
                cluster.instability,
                width = namespace_width
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
