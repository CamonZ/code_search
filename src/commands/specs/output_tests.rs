//! Output formatting tests for specs command.

#[cfg(test)]
mod tests {
    use super::super::execute::SpecEntry;
    use crate::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Specs: NonExistent

No specs found.";

    const SINGLE_TABLE: &str = "\
Specs: MyApp.Accounts.get_user

Found 1 spec(s) in 1 module(s):

MyApp.Accounts:
  get_user/1 [spec] L8
    @spec get_user(integer()) :: {:ok, User.t()} | {:error, :not_found}";

    const MULTIPLE_TABLE: &str = "\
Specs: MyApp.Accounts

Found 2 spec(s) in 1 module(s):

MyApp.Accounts:
  get_user/1 [spec] L8
    @spec get_user(integer()) :: {:ok, User.t()} | {:error, :not_found}
  list_users/0 [spec] L22
    @spec list_users() :: [User.t()]";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<SpecEntry> {
        ModuleCollectionResult {
            module_pattern: "NonExistent".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<SpecEntry> {
        ModuleCollectionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![SpecEntry {
                    name: "get_user".to_string(),
                    arity: 1,
                    kind: "spec".to_string(),
                    line: 8,
                    inputs: "integer()".to_string(),
                    returns: "{:ok, User.t()} | {:error, :not_found}".to_string(),
                    full: "@spec get_user(integer()) :: {:ok, User.t()} | {:error, :not_found}".to_string(),
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleCollectionResult<SpecEntry> {
        ModuleCollectionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![
                    SpecEntry {
                        name: "get_user".to_string(),
                        arity: 1,
                        kind: "spec".to_string(),
                        line: 8,
                        inputs: "integer()".to_string(),
                        returns: "{:ok, User.t()} | {:error, :not_found}".to_string(),
                        full: "@spec get_user(integer()) :: {:ok, User.t()} | {:error, :not_found}".to_string(),
                    },
                    SpecEntry {
                        name: "list_users".to_string(),
                        arity: 0,
                        kind: "spec".to_string(),
                        line: 22,
                        inputs: String::new(),
                        returns: "[User.t()]".to_string(),
                        full: "@spec list_users() :: [User.t()]".to_string(),
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
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: crate::test_utils::load_output_fixture("specs", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: crate::test_utils::load_output_fixture("specs", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<SpecEntry>,
        expected: crate::test_utils::load_output_fixture("specs", "empty.toon"),
        format: Toon,
    }
}
