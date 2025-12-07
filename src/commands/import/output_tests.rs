//! Output formatting tests for import command.

#[cfg(test)]
mod tests {
    use super::super::execute::{ImportResult, SchemaResult};
    use crate::output::OutputFormat;
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

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ImportResult,
        expected: EMPTY_TABLE_OUTPUT,
    }

    crate::output_table_test! {
        test_name: test_to_table_with_data,
        fixture: full_result,
        fixture_type: ImportResult,
        expected: FULL_TABLE_OUTPUT,
    }

    #[rstest]
    fn test_to_table_no_clear(full_result: ImportResult) {
        use crate::output::Outputable;
        let mut result = full_result;
        result.cleared = false;
        assert_eq!(result.to_table(), FULL_TABLE_OUTPUT_NO_CLEAR);
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: full_result,
        fixture_type: ImportResult,
        expected: FULL_JSON_OUTPUT,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: full_result,
        fixture_type: ImportResult,
        expected: FULL_TOON_OUTPUT,
        format: Toon,
    }

    #[rstest]
    fn test_format_table_delegates_to_to_table(full_result: ImportResult) {
        use crate::output::Outputable;
        assert_eq!(full_result.format(OutputFormat::Table), FULL_TABLE_OUTPUT);
    }
}
