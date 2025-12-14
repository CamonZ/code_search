//! Output formatting for setup command results.

use crate::output::Outputable;
use crate::commands::setup::execute::{SetupResult, RelationState};

impl Outputable for SetupResult {
    fn to_table(&self) -> String {
        let mut output = String::new();

        output.push_str("Database Setup\n\n");

        // Configuration section (if created)
        if self.config_file_created {
            output.push_str("Configuration\n\n");
            output.push_str("  Created .code_search.json\n");
            if let Some(path) = &self.config_file_path {
                output.push_str(&format!("    at: {}\n", path));
            }
            output.push_str("\n");
        }

        // Schema section
        output.push_str("Database Schema\n\n");

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
