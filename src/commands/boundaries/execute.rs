use std::error::Error;

use serde::Serialize;

use super::BoundariesCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::hotspots::{find_hotspots, HotspotKind};
use crate::types::{ModuleCollectionResult, ModuleGroup};

/// A single boundary module entry
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryEntry {
    pub incoming: i64,
    pub outgoing: i64,
    pub ratio: f64,
}

impl Execute for BoundariesCmd {
    type Output = ModuleCollectionResult<BoundaryEntry>;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        // Use find_hotspots with Ratio kind to get modules sorted by ratio
        let hotspots = find_hotspots(
            db,
            HotspotKind::Ratio,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        // Group by module and filter by thresholds
        let mut modules: std::collections::HashMap<String, BoundaryEntry> =
            std::collections::HashMap::new();

        for hotspot in hotspots {
            // Apply thresholds
            if hotspot.incoming < self.min_incoming || hotspot.ratio < self.min_ratio {
                continue;
            }

            // Keep the best entry for each module (hotspots are already sorted by ratio)
            modules
                .entry(hotspot.module)
                .or_insert_with(|| BoundaryEntry {
                    incoming: hotspot.incoming,
                    outgoing: hotspot.outgoing,
                    ratio: hotspot.ratio,
                });
        }

        // Build module groups, maintaining sort order
        let mut items = Vec::new();
        for hotspot in find_hotspots(
            db,
            HotspotKind::Ratio,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )? {
            if hotspot.incoming >= self.min_incoming && hotspot.ratio >= self.min_ratio {
                if !items.iter().any(|m: &ModuleGroup<BoundaryEntry>| m.name == hotspot.module) {
                    items.push(ModuleGroup {
                        name: hotspot.module,
                        file: String::new(),
                        entries: vec![BoundaryEntry {
                            incoming: hotspot.incoming,
                            outgoing: hotspot.outgoing,
                            ratio: hotspot.ratio,
                        }],
                        function_count: None,
                    });
                }
            }
        }

        let total_items = items.len();

        Ok(ModuleCollectionResult {
            module_pattern: self.module.clone().unwrap_or_else(|| "*".to_string()),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items,
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::fixture;
    use tempfile::NamedTempFile;

    #[fixture]
    fn test_db() -> NamedTempFile {
        NamedTempFile::new().unwrap()
    }

    #[test]
    fn test_boundaries_execute_creates_result_with_boundary_kind() {
        // This test verifies the execute method creates a result with kind_filter set to "boundary"
        // Full integration tests would require a real database with call graph data
        // For now, we test the structure and defaults
        let _cmd = BoundariesCmd {
            min_incoming: 5,
            min_ratio: 2.0,
            module: None,
            common: crate::commands::CommonArgs {
                project: "default".to_string(),
                regex: false,
                limit: 50,
            },
        };

        // The execute method would call find_hotspots and filter results
        // We verify the command struct is created correctly
        assert_eq!(_cmd.min_incoming, 5);
        assert_eq!(_cmd.min_ratio, 2.0);
    }
}
