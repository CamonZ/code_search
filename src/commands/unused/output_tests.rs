//! Output formatting tests for unused command.

#[cfg(test)]
mod tests {
    use super::super::execute::{UnusedFunction, UnusedResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Unused functions in project 'test_project'

No unused functions found.";

    const SINGLE_TABLE: &str = "\
Unused functions in project 'test_project'

Found 1 unused function(s):
  [defp] MyApp.Accounts.unused_helper/0
       lib/accounts.ex:35";

    const FILTERED_TABLE: &str = "\
Unused functions in project 'test_project' (module: Accounts)

Found 1 unused function(s):
  [defp] MyApp.Accounts.unused_helper/0
       lib/accounts.ex:35";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![],
        }
    }

    #[fixture]
    fn single_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![UnusedFunction {
                module: "MyApp.Accounts".to_string(),
                name: "unused_helper".to_string(),
                arity: 0,
                kind: "defp".to_string(),
                file: "lib/accounts.ex".to_string(),
                line: 35,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: Some("Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![UnusedFunction {
                module: "MyApp.Accounts".to_string(),
                name: "unused_helper".to_string(),
                arity: 0,
                kind: "defp".to_string(),
                file: "lib/accounts.ex".to_string(),
                line: 35,
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: UnusedResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: UnusedResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_filtered,
        fixture: filtered_result,
        fixture_type: UnusedResult,
        expected: FILTERED_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: UnusedResult,
        expected: crate::test_utils::load_output_fixture("unused", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: UnusedResult,
        expected: crate::test_utils::load_output_fixture("unused", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: UnusedResult,
        expected: crate::test_utils::load_output_fixture("unused", "empty.toon"),
        format: Toon,
    }
}
