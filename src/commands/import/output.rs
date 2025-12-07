//! Output formatting for import command results.

use crate::output::Outputable;
use super::execute::ImportResult;

impl Outputable for ImportResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Schema info
        if !self.schemas.created.is_empty() {
            lines.push(format!("Schemas created: {}", self.schemas.created.join(", ")));
        }
        if !self.schemas.already_existed.is_empty() {
            lines.push(format!("Schemas existed:  {}", self.schemas.already_existed.join(", ")));
        }

        // Clear status
        if self.cleared {
            lines.push("Cleared existing project data".to_string());
        }

        // Import counts
        lines.push(String::new());
        lines.push("Import Summary:".to_string());
        lines.push(format!("  Modules:            {:>6}", self.modules_imported));
        lines.push(format!("  Functions:          {:>6}", self.functions_imported));
        lines.push(format!("  Calls:              {:>6}", self.calls_imported));
        lines.push(format!("  Struct fields:      {:>6}", self.structs_imported));
        lines.push(format!("  Function locations: {:>6}", self.function_locations_imported));

        let total = self.modules_imported
            + self.functions_imported
            + self.calls_imported
            + self.structs_imported
            + self.function_locations_imported;
        lines.push(format!("  Total:              {:>6}", total));

        lines.join("\n")
    }
}
