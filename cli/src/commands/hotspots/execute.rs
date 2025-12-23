use std::error::Error;

use serde::Serialize;

use super::HotspotsCmd;
use crate::commands::Execute;
use crate::output::Outputable;
use db::queries::hotspots::find_hotspots;

/// A function hotspot entry
#[derive(Debug, Clone, Serialize)]
pub struct FunctionHotspotEntry {
    pub module: String,
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
    pub ratio: f64,
}

/// Result type for hotspots command
#[derive(Debug, Serialize)]
pub struct HotspotsResult {
    pub kind: String,
    pub total_items: usize,
    pub entries: Vec<FunctionHotspotEntry>,
}

impl Outputable for HotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Hotspots ({})", self.kind));
        lines.push(String::new());

        if self.entries.is_empty() {
            lines.push("No hotspots found.".to_string());
            return lines.join("\n");
        }

        let item_word = if self.total_items == 1 {
            "function"
        } else {
            "functions"
        };
        lines.push(format!("Found {} {}:", self.total_items, item_word));
        lines.push(String::new());

        // Calculate column widths for alignment
        let name_width = self
            .entries
            .iter()
            .map(|e| e.module.len() + 1 + e.function.len())
            .max()
            .unwrap_or(0);
        let in_width = self
            .entries
            .iter()
            .map(|e| e.incoming.to_string().len())
            .max()
            .unwrap_or(0);
        let out_width = self
            .entries
            .iter()
            .map(|e| e.outgoing.to_string().len())
            .max()
            .unwrap_or(0);
        let total_width = self
            .entries
            .iter()
            .map(|e| e.total.to_string().len())
            .max()
            .unwrap_or(0);

        for entry in &self.entries {
            let name = format!("{}.{}", entry.module, entry.function);
            let ratio_str = if entry.ratio >= 9999.0 {
                "∞".to_string()
            } else {
                format!("{:.2}", entry.ratio)
            };
            lines.push(format!(
                "{:<name_width$}  {:>in_width$} in  {:>out_width$} out  {:>total_width$} total  {:>6} ratio",
                name,
                entry.incoming,
                entry.outgoing,
                entry.total,
                ratio_str,
                name_width = name_width,
                in_width = in_width,
                out_width = out_width,
                total_width = total_width,
            ));
        }

        lines.join("\n")
    }
}

impl Execute for HotspotsCmd {
    type Output = HotspotsResult;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let hotspots = find_hotspots(
            db,
            self.kind,
            self.module.as_deref(),
            &self.common.project,
            self.common.regex,
            self.common.limit,
            self.exclude_generated,
            false, // Don't require outgoing calls
        )?;

        let kind_str = match self.kind {
            db::queries::hotspots::HotspotKind::Incoming => "incoming",
            db::queries::hotspots::HotspotKind::Outgoing => "outgoing",
            db::queries::hotspots::HotspotKind::Total => "total",
            db::queries::hotspots::HotspotKind::Ratio => "ratio",
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

        Ok(HotspotsResult {
            kind: kind_str.to_string(),
            total_items,
            entries,
        })
    }
}
