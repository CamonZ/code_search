use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::output::Outputable;
use crate::queries::hotspots::{find_hotspots, get_function_counts, HotspotKind};

/// A function hotspot entry (for flat list display)
#[derive(Debug, Clone, Serialize)]
pub struct FunctionHotspotEntry {
    pub module: String,
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
    pub ratio: f64,
}

/// A module with function count (for module-level display)
#[derive(Debug, Clone, Serialize)]
pub struct ModuleCountEntry {
    pub module: String,
    pub count: i64,
}

/// Result type for hotspots command - can be either function-level or module-level
#[derive(Debug, Serialize)]
pub enum HotspotsResult {
    Functions(FunctionHotspotsResult),
    Modules(ModuleHotspotsResult),
}

/// Function-level hotspots (flat list)
#[derive(Debug, Serialize)]
pub struct FunctionHotspotsResult {
    pub kind: String,
    pub module_pattern: String,
    pub total_items: usize,
    pub entries: Vec<FunctionHotspotEntry>,
}

/// Module-level hotspots (module counts)
#[derive(Debug, Serialize)]
pub struct ModuleHotspotsResult {
    pub kind: String,
    pub module_pattern: String,
    pub total_items: usize,
    pub entries: Vec<ModuleCountEntry>,
}

impl Outputable for HotspotsResult {
    fn to_table(&self) -> String {
        match self {
            HotspotsResult::Functions(result) => result.to_table(),
            HotspotsResult::Modules(result) => result.to_table(),
        }
    }
}

impl Outputable for FunctionHotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Hotspots ({})", self.kind));
        lines.push(String::new());

        if self.entries.is_empty() {
            lines.push("No hotspots found.".to_string());
            return lines.join("\n");
        }

        let item_word = if self.total_items == 1 { "function" } else { "function(s)" };
        lines.push(format!("Found {} {}:", self.total_items, item_word));
        lines.push(String::new());

        for entry in &self.entries {
            lines.push(format!(
                "{}.{}    in: {}  out: {}  total: {}  ratio: {:.2}",
                entry.module, entry.function, entry.incoming, entry.outgoing, entry.total, entry.ratio
            ));
        }

        lines.join("\n")
    }
}

impl Outputable for ModuleHotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Hotspots ({})", self.kind));
        lines.push(String::new());

        if self.entries.is_empty() {
            lines.push("No hotspots found.".to_string());
            return lines.join("\n");
        }

        let item_word = if self.total_items == 1 { "module" } else { "module(s)" };
        lines.push(format!("Found {} {}:", self.total_items, item_word));
        lines.push(String::new());

        for entry in &self.entries {
            let count_word = if entry.count == 1 { "function" } else { "functions" };
            lines.push(format!("{:<42}  {} {}", entry.module, entry.count, count_word));
        }

        lines.join("\n")
    }
}

impl Execute for HotspotsCmd {
    type Output = HotspotsResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        if matches!(self.kind, HotspotKind::Functions) {
            // Module-level: get function counts
            let func_counts = get_function_counts(
                db,
                &self.common.project,
                self.module.as_deref(),
                self.common.regex,
            )?;

            // Sort by count descending
            let mut entries: Vec<_> = func_counts
                .into_iter()
                .map(|(module, count)| ModuleCountEntry { module, count })
                .collect();
            entries.sort_by(|a, b| b.count.cmp(&a.count));

            let limit = self.common.limit as usize;
            let total_items = entries.len();
            entries.truncate(limit);

            Ok(HotspotsResult::Modules(ModuleHotspotsResult {
                kind: "functions".to_string(),
                module_pattern: self.module.unwrap_or_else(|| "*".to_string()),
                total_items,
                entries,
            }))
        } else {
            // Function-level: get hotspots
            let hotspots = find_hotspots(
                db,
                self.kind,
                self.module.as_deref(),
                &self.common.project,
                self.common.regex,
                self.common.limit,
            )?;

            let kind_str = match self.kind {
                HotspotKind::Incoming => "incoming".to_string(),
                HotspotKind::Outgoing => "outgoing".to_string(),
                HotspotKind::Total => "total".to_string(),
                HotspotKind::Ratio => "ratio".to_string(),
                HotspotKind::Functions => unreachable!(),
            };

            let entries: Vec<FunctionHotspotEntry> = hotspots
                .into_iter()
                .map(|hotspot| FunctionHotspotEntry {
                    module: hotspot.module,
                    function: hotspot.function,
                    incoming: hotspot.incoming,
                    outgoing: hotspot.outgoing,
                    total: hotspot.total,
                    ratio: hotspot.ratio,
                })
                .collect();

            let total_items = entries.len();

            Ok(HotspotsResult::Functions(FunctionHotspotsResult {
                kind: kind_str,
                module_pattern: self.module.unwrap_or_else(|| "*".to_string()),
                total_items,
                entries,
            }))
        }
    }
}
