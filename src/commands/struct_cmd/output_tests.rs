//! Output formatting tests for struct command.

#[cfg(test)]
mod tests {
    use super::super::execute::{FieldInfo, StructDefinition, StructResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Struct: MyApp.User

No structs found.";

    const SINGLE_TABLE: &str = "\
Struct: MyApp.User

Found 1 struct(s):

  [default] MyApp.User
    *id :: integer() \\ nil
    name :: String.t() \\ nil";

    const MULTIPLE_TABLE: &str = "\
Struct: MyApp.*

Found 2 struct(s):

  [default] MyApp.User
    *id :: integer() \\ nil

  [default] MyApp.Post
    *title :: String.t() \\ nil";

    const SINGLE_JSON: &str = r#"{
  "module_pattern": "MyApp.User",
  "structs": [
    {
      "project": "default",
      "module": "MyApp.User",
      "fields": [
        {
          "name": "id",
          "default_value": "nil",
          "required": true,
          "inferred_type": "integer()"
        },
        {
          "name": "name",
          "default_value": "nil",
          "required": false,
          "inferred_type": "String.t()"
        }
      ]
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
module_pattern: MyApp.User
structs[1]:
  - fields[2]{default_value,inferred_type,name,required}:
    nil,integer(),id,true
    nil,String.t(),name,false
    module: MyApp.User
    project: default";

    const EMPTY_TOON: &str = "\
module_pattern: MyApp.User
structs[0]:";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.User".to_string(),
            structs: vec![],
        }
    }

    #[fixture]
    fn single_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.User".to_string(),
            structs: vec![StructDefinition {
                project: "default".to_string(),
                module: "MyApp.User".to_string(),
                fields: vec![
                    FieldInfo {
                        name: "id".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "integer()".to_string(),
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        default_value: "nil".to_string(),
                        required: false,
                        inferred_type: "String.t()".to_string(),
                    },
                ],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.*".to_string(),
            structs: vec![
                StructDefinition {
                    project: "default".to_string(),
                    module: "MyApp.User".to_string(),
                    fields: vec![FieldInfo {
                        name: "id".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "integer()".to_string(),
                    }],
                },
                StructDefinition {
                    project: "default".to_string(),
                    module: "MyApp.Post".to_string(),
                    fields: vec![FieldInfo {
                        name: "title".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "String.t()".to_string(),
                    }],
                },
            ],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: StructResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: StructResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: StructResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: StructResult,
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: StructResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: StructResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
