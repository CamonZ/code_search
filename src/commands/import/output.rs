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
        output.push_str(&format!("  Specs: {}\n", self.specs_imported));
        output.push_str(&format!("  Types: {}\n", self.types_imported));

        output
    }
}
