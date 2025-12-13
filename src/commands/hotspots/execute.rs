use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::queries::hotspots::{find_hotspots, get_function_counts, Hotspot, HotspotKind};
use crate::types::{ModuleCollectionResult, ModuleGroup};

/// A single hotspot entry (function within a module)
#[derive(Debug, Clone, Serialize)]
pub struct HotspotEntry {
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
    pub ratio: f64,
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
                ratio: hotspot.ratio,
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
            HotspotKind::Ratio => "ratio".to_string(),
            HotspotKind::Functions => "functions".to_string(),
        };

        // For Functions kind, skip hotspot query and go straight to function counts
        let mut result = if matches!(self.kind, HotspotKind::Functions) {
            // Create an empty result for Functions kind (counts will be added below)
            ModuleCollectionResult {
                module_pattern: self.module.clone().unwrap_or_else(|| "*".to_string()),
                function_pattern: None,
                kind_filter: Some(kind_str),
                name_filter: None,
                total_items: 0,
                items: vec![],
            }
        } else {
            let hotspots = find_hotspots(
                db,
                self.kind,
                self.module.as_deref(),
                &self.common.project,
                self.common.regex,
                self.common.limit,
            )?;

            <ModuleCollectionResult<HotspotEntry>>::from_hotspots(
                self.module.clone().unwrap_or_else(|| "*".to_string()),
                kind_str,
                hotspots,
            )
        };

        // Add function counts for all modules
        let func_counts = if matches!(self.kind, HotspotKind::Functions) {
            get_function_counts(
                db,
                &self.common.project,
                self.module.as_deref(),
                self.common.regex,
            )?
        } else {
            // For other kinds, we don't need function counts yet
            std::collections::HashMap::new()
        };

        // For Functions kind, convert function counts into module entries
        if matches!(self.kind, HotspotKind::Functions) {
            // Create module groups from function counts, sorted by count
            let mut modules_with_counts: Vec<_> = func_counts.iter().collect();
            modules_with_counts.sort_by(|a, b| b.1.cmp(a.1)); // descending by count

            let limit = self.common.limit as usize;
            for (module_name, count) in modules_with_counts.into_iter().take(limit) {
                result.items.push(ModuleGroup {
                    name: module_name.clone(),
                    file: String::new(),
                    entries: vec![],
                    function_count: Some(*count),
                });
                result.total_items += 1;
            }
        } else {
            // For other kinds, add function counts to existing modules
            for module in &mut result.items {
                if let Some(&count) = func_counts.get(&module.name) {
                    module.function_count = Some(count);
                }
            }
        }

        Ok(result)
    }
}
