//! Output formatting tests for types command.

#[cfg(test)]
mod tests {
    use super::super::execute::TypeEntry;
    use crate::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Types: NonExistent

No types found.";

    const SINGLE_TABLE: &str = "\
Types: MyApp.Accounts.user

Found 1 type(s) in 1 module(s):

MyApp.Accounts:
  user [type] L5
    @type user() :: %{id: integer(), name: String.t()}";

    const MULTIPLE_TABLE: &str = "\
Types: MyApp.Accounts

Found 2 type(s) in 1 module(s):

MyApp.Accounts:
  user [type] L5
    @type user() :: %{id: integer(), name: String.t()}
  user_id [opaque] L3
    @opaque user_id() :: integer()";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<TypeEntry> {
        ModuleCollectionResult {
            module_pattern: "NonExistent".to_string(),
            function_pattern: None,
            name_filter: None,
            kind_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<TypeEntry> {
        ModuleCollectionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: None,
            name_filter: Some("user".to_string()),
            kind_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![TypeEntry {
                    name: "user".to_string(),
                    kind: "type".to_string(),
                    params: String::new(),
                    line: 5,
                    definition: "@type user() :: %{id: integer(), name: String.t()}".to_string(),
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleCollectionResult<TypeEntry> {
        ModuleCollectionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: None,
            name_filter: None,
            kind_filter: None,
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![
                    TypeEntry {
                        name: "user".to_string(),
                        kind: "type".to_string(),
                        params: String::new(),
                        line: 5,
                        definition: "@type user() :: %{id: integer(), name: String.t()}".to_string(),
                    },
                    TypeEntry {
                        name: "user_id".to_string(),
                        kind: "opaque".to_string(),
                        params: String::new(),
                        line: 3,
                        definition: "@opaque user_id() :: integer()".to_string(),
                    },
                ],
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: crate::test_utils::load_output_fixture("types", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: crate::test_utils::load_output_fixture("types", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<TypeEntry>,
        expected: crate::test_utils::load_output_fixture("types", "empty.toon"),
        format: Toon,
    }
}
