use std::error::Error;

use serde::Serialize;

use super::BoundariesCmd;
use crate::commands::Execute;
use db::queries::hotspots::{find_hotspots, HotspotKind};
use db::types::{ModuleCollectionResult, ModuleGroup};

/// A single boundary module entry
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryEntry {
    pub incoming: i64,
    pub outgoing: i64,
    pub ratio: f64,
}

impl Execute for BoundariesCmd {
    type Output = ModuleCollectionResult<BoundaryEntry>;

    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>> {
        let hotspots = find_hotspots(
            db,
            HotspotKind::Ratio,
            self.module.as_deref(),
            self.common.regex,
            self.common.limit,
            false,
            true, // require_outgoing: exclude leaf nodes
        )?;

        // Build module groups, filtering by thresholds and deduplicating by module
        let mut seen_modules = std::collections::HashSet::new();
        let mut items = Vec::new();

        for hotspot in hotspots {
            // Boundaries must have both incoming AND outgoing calls
            // (leaf modules with only incoming calls are not boundaries)
            if hotspot.incoming >= self.min_incoming
                && hotspot.outgoing >= 1
                && hotspot.ratio >= self.min_ratio
                && seen_modules.insert(hotspot.module.clone())
            {
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

        let total_items = items.len();

        Ok(ModuleCollectionResult {
            module_pattern: self.module.unwrap_or_else(|| "*".to_string()),
            function_pattern: None,
            kind_filter: Some("boundary".to_string()),
            name_filter: None,
            total_items,
            items,
        })
    }
}
