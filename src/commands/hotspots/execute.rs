use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::queries::hotspots::{find_hotspots, Hotspot, HotspotKind};
use crate::types::ModuleCollectionResult;

/// A single hotspot entry (function within a module)
#[derive(Debug, Clone, Serialize)]
pub struct HotspotEntry {
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
}

impl ModuleCollectionResult<HotspotEntry> {
    /// Build grouped result from flat Hotspot list
    fn from_hotspots(
        module_pattern: String,
        kind_filter: String,
        hotspots: Vec<Hotspot>,
    ) -> Self {
        let total_items = hotspots.len();

        // Use helper to group by module
        let items = crate::utils::group_by_module(hotspots, |hotspot| {
            let entry = HotspotEntry {
                function: hotspot.function,
                incoming: hotspot.incoming,
                outgoing: hotspot.outgoing,
                total: hotspot.total,
            };
            (hotspot.module, entry)
        });

        ModuleCollectionResult {
            module_pattern,
            function_pattern: None,
            kind_filter: Some(kind_filter),
            name_filter: None,
            total_items,
            items,
        }
    }
}

impl Execute for HotspotsCmd {
    type Output = ModuleCollectionResult<HotspotEntry>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let kind_str = match self.kind {
            HotspotKind::Incoming => "incoming".to_string(),
            HotspotKind::Outgoing => "outgoing".to_string(),
            HotspotKind::Total => "total".to_string(),
        };

        let hotspots = find_hotspots(
            db,
            self.kind,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(<ModuleCollectionResult<HotspotEntry>>::from_hotspots(
            self.module.unwrap_or_else(|| "*".to_string()),
            kind_str,
            hotspots,
        ))
    }
}
