use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::queries::hotspots::{find_hotspots, Hotspot, HotspotKind};

/// A single hotspot entry (function within a module)
#[derive(Debug, Clone, Serialize)]
pub struct HotspotEntry {
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
}

/// A module containing hotspot functions
#[derive(Debug, Clone, Serialize)]
pub struct HotspotModule {
    pub name: String,
    pub functions: Vec<HotspotEntry>,
}

/// Result of the hotspots command execution
#[derive(Debug, Default, Serialize)]
pub struct HotspotsResult {
    pub project: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_filter: Option<String>,
    pub total_hotspots: usize,
    pub modules: Vec<HotspotModule>,
}

impl HotspotsResult {
    /// Build grouped result from flat Hotspot list
    fn from_hotspots(
        project: String,
        kind: String,
        module_filter: Option<String>,
        hotspots: Vec<Hotspot>,
    ) -> Self {
        let total_hotspots = hotspots.len();

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

        let modules: Vec<HotspotModule> = module_map
            .into_iter()
            .map(|(name, functions)| HotspotModule { name, functions })
            .collect();

        HotspotsResult {
            project,
            kind,
            module_filter,
            total_hotspots,
            modules,
        }
    }
}

impl Execute for HotspotsCmd {
    type Output = HotspotsResult;

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

        Ok(HotspotsResult::from_hotspots(
            self.project,
            kind_str,
            self.module,
            hotspots,
        ))
    }
}
