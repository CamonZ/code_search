use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::queries::hotspots::{find_hotspots, Hotspot, HotspotKind};
use crate::types::{ModuleCollectionResult, ModuleGroup};

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

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<HotspotEntry>> = BTreeMap::new();

        for hotspot in hotspots {
            let entry = HotspotEntry {
                function: hotspot.function,
                incoming: hotspot.incoming,
                outgoing: hotspot.outgoing,
                total: hotspot.total,
            };

            module_map.entry(hotspot.module).or_default().push(entry);
        }

        let items: Vec<ModuleGroup<HotspotEntry>> = module_map
            .into_iter()
            .map(|(name, entries)| ModuleGroup {
                name,
                file: String::new(),
                entries,
            })
            .collect();

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
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(<ModuleCollectionResult<HotspotEntry>>::from_hotspots(
            self.module.unwrap_or_else(|| "*".to_string()),
            kind_str,
            hotspots,
        ))
    }
}
