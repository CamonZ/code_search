//! Output formatting tests for unused command.

#[cfg(test)]
mod tests {
    use super::super::execute::UnusedFunc;
    use crate::types::{ModuleCollectionResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Unused functions

No unused functions found.";

    const SINGLE_TABLE: &str = "\
Unused functions

Found 1 unused function(s) in 1 module(s):

MyApp.Accounts (lib/accounts.ex):
  unused_helper/0 [defp] L35";

    const FILTERED_TABLE: &str = "\
Unused functions (module: Accounts)

Found 1 unused function(s) in 1 module(s):

MyApp.Accounts (lib/accounts.ex):
  unused_helper/0 [defp] L35";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleCollectionResult<UnusedFunc> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleCollectionResult<UnusedFunc> {
        ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/accounts.ex".to_string(),
                entries: vec![UnusedFunc {
                    name: "unused_helper".to_string(),
                    arity: 0,
                    kind: "defp".to_string(),
                    line: 35,
                }],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> ModuleCollectionResult<UnusedFunc> {
        ModuleCollectionResult {
            module_pattern: "Accounts".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/accounts.ex".to_string(),
                entries: vec![UnusedFunc {
                    name: "unused_helper".to_string(),
                    arity: 0,
                    kind: "defp".to_string(),
                    line: 35,
                }],
                function_count: None,
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_filtered,
        fixture: filtered_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: FILTERED_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: crate::test_utils::load_output_fixture("unused", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: crate::test_utils::load_output_fixture("unused", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleCollectionResult<UnusedFunc>,
        expected: crate::test_utils::load_output_fixture("unused", "empty.toon"),
        format: Toon,
    }
}
