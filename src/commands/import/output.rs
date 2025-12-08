//! Output formatting for import command results.

use crate::output::Outputable;
use crate::queries::import::ImportResult;

impl Outputable for ImportResult {
    fn to_table(&self) -> String {
        let mut output = String::new();
        
        if self.cleared {
            output.push_str("Cleared existing project data.\n\n");
        }

        output.push_str("Import Summary:\n");
        output.push_str(&format!("  Modules: {}\n", self.modules_imported));
        output.push_str(&format!("  Functions: {}\n", self.functions_imported));
        output.push_str(&format!("  Calls: {}\n", self.calls_imported));
        output.push_str(&format!("  Structs: {}\n", self.structs_imported));
        output.push_str(&format!("  Locations: {}\n", self.function_locations_imported));

        if !self.schemas.created.is_empty() {
            output.push_str("\nCreated Schemas:\n");
            for schema in &self.schemas.created {
                output.push_str(&format!("  - {}\n", schema));
            }
        }

        output
    }
}
