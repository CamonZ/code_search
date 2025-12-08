use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::queries::hotspots::{find_hotspots, Hotspot, HotspotKind};

/// Result of the hotspots command execution
#[derive(Debug, Default, Serialize)]
pub struct HotspotsResult {
    pub project: String,
    pub kind: String,
    pub module_filter: Option<String>,
    pub hotspots: Vec<Hotspot>,
}

impl Execute for HotspotsCmd {
    type Output = HotspotsResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = HotspotsResult {
            project: self.project.clone(),
            kind: match self.kind {
                HotspotKind::Incoming => "incoming".to_string(),
                HotspotKind::Outgoing => "outgoing".to_string(),
                HotspotKind::Total => "total".to_string(),
            },
            module_filter: self.module.clone(),
            ..Default::default()
        };

        result.hotspots = find_hotspots(
            db,
            self.kind,
            self.module.as_deref(),
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}
