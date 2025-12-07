//! Output formatting for command results.
//!
//! Supports multiple output formats: table (human-readable), JSON, and toon.

use clap::ValueEnum;
use serde::Serialize;

/// Output format for command results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format
    Json,
    /// Token-efficient toon format
    Toon,
}

/// Trait for types that can be formatted for output
pub trait Outputable: Serialize {
    /// Format as a human-readable table
    fn to_table(&self) -> String;

    /// Format according to the specified output format
    fn format(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Table => self.to_table(),
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
            OutputFormat::Toon => {
                let json_value = serde_json::to_value(self).unwrap_or_default();
                toon::encode(&json_value, None)
            }
        }
    }
}
