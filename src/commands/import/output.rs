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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use super::super::execute::SchemaResult;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "
Import Summary:
  Modules:                 0
  Functions:               0
  Calls:                   0
  Struct fields:           0
  Function locations:      0
  Total:                   0";

    const FULL_TABLE_OUTPUT: &str = "\
Schemas created: modules, functions
Schemas existed:  calls
Cleared existing project data

Import Summary:
  Modules:                10
  Functions:              50
  Calls:                 100
  Struct fields:           5
  Function locations:     45
  Total:                 210";

    const FULL_TABLE_OUTPUT_NO_CLEAR: &str = "\
Schemas created: modules, functions
Schemas existed:  calls

Import Summary:
  Modules:                10
  Functions:              50
  Calls:                 100
  Struct fields:           5
  Function locations:     45
  Total:                 210";

    const FULL_JSON_OUTPUT: &str = r#"{
  "schemas": {
    "created": [
      "modules",
      "functions"
    ],
    "already_existed": [
      "calls"
    ]
  },
  "cleared": true,
  "modules_imported": 10,
  "functions_imported": 50,
  "calls_imported": 100,
  "structs_imported": 5,
  "function_locations_imported": 45
}"#;

    const FULL_TOON_OUTPUT: &str = "\
calls_imported: 100
cleared: true
function_locations_imported: 45
functions_imported: 50
modules_imported: 10
schemas:
  already_existed[1]: calls
  created[2]: modules,functions
structs_imported: 5";

    #[fixture]
    fn empty_result() -> ImportResult {
        ImportResult::default()
    }

    #[fixture]
    fn full_result() -> ImportResult {
        ImportResult {
            schemas: SchemaResult {
                created: vec!["modules".to_string(), "functions".to_string()],
                already_existed: vec!["calls".to_string()],
            },
            cleared: true,
            modules_imported: 10,
            functions_imported: 50,
            calls_imported: 100,
            structs_imported: 5,
            function_locations_imported: 45,
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: ImportResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_with_data(full_result: ImportResult) {
        assert_eq!(full_result.to_table(), FULL_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_no_clear(full_result: ImportResult) {
        let mut result = full_result;
        result.cleared = false;
        assert_eq!(result.to_table(), FULL_TABLE_OUTPUT_NO_CLEAR);
    }

    #[rstest]
    fn test_format_json(full_result: ImportResult) {
        assert_eq!(full_result.format(OutputFormat::Json), FULL_JSON_OUTPUT);
    }

    #[rstest]
    fn test_format_toon(full_result: ImportResult) {
        assert_eq!(full_result.format(OutputFormat::Toon), FULL_TOON_OUTPUT);
    }

    #[rstest]
    fn test_format_table_delegates_to_to_table(full_result: ImportResult) {
        assert_eq!(full_result.format(OutputFormat::Table), FULL_TABLE_OUTPUT);
    }
}
