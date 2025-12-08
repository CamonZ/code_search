//! Output formatting tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::execute::DependsOnResult;
    use crate::queries::depends_on::ModuleDependency;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Dependencies of: MyApp.Controller

No dependencies found.";

    const SINGLE_TABLE: &str = "\
Dependencies of: MyApp.Controller

Found 1 module(s):
  MyApp.Service (5 calls)";

    const MULTIPLE_TABLE: &str = "\
Dependencies of: MyApp.Controller

Found 2 module(s):
  MyApp.Service (5 calls)
  Phoenix.View (2 calls)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![ModuleDependency {
                module: "MyApp.Service".to_string(),
                call_count: 5,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![
                ModuleDependency {
                    module: "MyApp.Service".to_string(),
                    call_count: 5,
                },
                ModuleDependency {
                    module: "Phoenix.View".to_string(),
                    call_count: 2,
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
        fixture_type: DependsOnResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: DependsOnResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: DependsOnResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: DependsOnResult,
        expected: crate::test_utils::load_output_fixture("depends_on", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: DependsOnResult,
        expected: crate::test_utils::load_output_fixture("depends_on", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: DependsOnResult,
        expected: crate::test_utils::load_output_fixture("depends_on", "empty.toon"),
        format: Toon,
    }
}
