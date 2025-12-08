//! Output formatting tests for search command.

#[cfg(test)]
mod tests {
    use super::super::execute::SearchResult;
    use crate::queries::search::{FunctionResult, ModuleResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Search: test (modules)

No results found.";

    const MODULES_TABLE: &str = "\
Search: MyApp (modules)

Modules (2):
  MyApp.Accounts
  MyApp.Users";

    const FUNCTIONS_TABLE: &str = "\
Search: get_ (functions)

Functions (1):
  MyApp.Accounts.get_user/1 -> User.t()";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> SearchResult {
        SearchResult {
            pattern: "test".to_string(),
            kind: "modules".to_string(),
            modules: vec![],
            functions: vec![],
        }
    }

    #[fixture]
    fn modules_result() -> SearchResult {
        SearchResult {
            pattern: "MyApp".to_string(),
            kind: "modules".to_string(),
            modules: vec![
                ModuleResult {
                    project: "default".to_string(),
                    name: "MyApp.Accounts".to_string(),
                    source: "unknown".to_string(),
                },
                ModuleResult {
                    project: "default".to_string(),
                    name: "MyApp.Users".to_string(),
                    source: "unknown".to_string(),
                },
            ],
            functions: vec![],
        }
    }

    #[fixture]
    fn functions_result() -> SearchResult {
        SearchResult {
            pattern: "get_".to_string(),
            kind: "functions".to_string(),
            modules: vec![],
            functions: vec![FunctionResult {
                project: "default".to_string(),
                module: "MyApp.Accounts".to_string(),
                name: "get_user".to_string(),
                arity: 1,
                return_type: "User.t()".to_string(),
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: SearchResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_modules,
        fixture: modules_result,
        fixture_type: SearchResult,
        expected: MODULES_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_functions,
        fixture: functions_result,
        fixture_type: SearchResult,
        expected: FUNCTIONS_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: modules_result,
        fixture_type: SearchResult,
        expected: crate::test_utils::load_output_fixture("search", "modules.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: modules_result,
        fixture_type: SearchResult,
        expected: crate::test_utils::load_output_fixture("search", "modules.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: SearchResult,
        expected: crate::test_utils::load_output_fixture("search", "empty.toon"),
        format: Toon,
    }
}
