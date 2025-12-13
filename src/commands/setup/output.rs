//! Output formatting for setup command results.

use crate::output::Outputable;
use crate::commands::setup::execute::{SetupResult, RelationState};

impl Outputable for SetupResult {
    fn to_table(&self) -> String {
        let mut output = String::new();

        output.push_str("Database Setup\n\n");

        if self.dry_run {
            output.push_str("Schema creation (dry-run):\n");
        } else {
            output.push_str("Schema creation:\n");
        }

        for relation in &self.relations {
            let symbol = match relation.status {
                RelationState::Created => "✓",
                RelationState::AlreadyExists => "✓",
                RelationState::WouldCreate => "→",
            };

            let status_text = match relation.status {
                RelationState::Created => "created",
                RelationState::AlreadyExists => "exists",
                RelationState::WouldCreate => "would create",
            };

            output.push_str(&format!("  {} {} ({})\n", symbol, relation.name, status_text));
        }

        if self.dry_run {
            output.push_str("\nNo changes made (dry-run mode).\n");
        } else if self.created_new {
            output.push_str("\nDatabase ready.\n");
        } else {
            output.push_str("\nDatabase already configured.\n");
        }

        output
    }
}
