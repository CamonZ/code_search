//! Output formatting tests for types command.

#[cfg(test)]
mod tests {
    use super::super::execute::TypesResult;
    use crate::queries::types::TypeInfo;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Types: NonExistent

No types found.";

    const SINGLE_TABLE: &str = "\
Types: MyApp.Accounts.user

Found 1 type(s):
  MyApp.Accounts.user [type] line 5
       @type user() :: %{id: integer(), name: String.t()}";

    const MULTIPLE_TABLE: &str = "\
Types: MyApp.Accounts

Found 2 type(s):
  MyApp.Accounts.user [type] line 5
       @type user() :: %{id: integer(), name: String.t()}
  MyApp.Accounts.user_id [opaque] line 3
       @opaque user_id() :: integer()";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> TypesResult {
        TypesResult {
            module_pattern: "NonExistent".to_string(),
            name_filter: None,
            kind_filter: None,
            types: vec![],
        }
    }

    #[fixture]
    fn single_result() -> TypesResult {
        TypesResult {
            module_pattern: "MyApp.Accounts".to_string(),
            name_filter: Some("user".to_string()),
            kind_filter: None,
            types: vec![TypeInfo {
                project: "default".to_string(),
                module: "MyApp.Accounts".to_string(),
                name: "user".to_string(),
                kind: "type".to_string(),
                params: String::new(),
                line: 5,
                definition: "@type user() :: %{id: integer(), name: String.t()}".to_string(),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> TypesResult {
        TypesResult {
            module_pattern: "MyApp.Accounts".to_string(),
            name_filter: None,
            kind_filter: None,
            types: vec![
                TypeInfo {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "user".to_string(),
                    kind: "type".to_string(),
                    params: String::new(),
                    line: 5,
                    definition: "@type user() :: %{id: integer(), name: String.t()}".to_string(),
                },
                TypeInfo {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "user_id".to_string(),
                    kind: "opaque".to_string(),
                    params: String::new(),
                    line: 3,
                    definition: "@opaque user_id() :: integer()".to_string(),
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
        fixture_type: TypesResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: TypesResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: TypesResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: TypesResult,
        expected: crate::test_utils::load_output_fixture("types", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: TypesResult,
        expected: crate::test_utils::load_output_fixture("types", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: TypesResult,
        expected: crate::test_utils::load_output_fixture("types", "empty.toon"),
        format: Toon,
    }
}
