//! Output formatting tests for depended-by command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DependedByResult, ModuleDependent};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Modules that depend on: MyApp.Repo

No dependents found.";

    const SINGLE_TABLE: &str = "\
Modules that depend on: MyApp.Repo

Found 1 module(s):
  MyApp.Service (3 calls)";

    const MULTIPLE_TABLE: &str = "\
Modules that depend on: MyApp.Repo

Found 2 module(s):
  MyApp.Service (5 calls)
  MyApp.Controller (2 calls)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![ModuleDependent {
                module: "MyApp.Service".to_string(),
                call_count: 3,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![
                ModuleDependent {
                    module: "MyApp.Service".to_string(),
                    call_count: 5,
                },
                ModuleDependent {
                    module: "MyApp.Controller".to_string(),
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
        fixture_type: DependedByResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: DependedByResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: DependedByResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: DependedByResult,
        expected: crate::test_utils::load_output_fixture("depended_by", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: DependedByResult,
        expected: crate::test_utils::load_output_fixture("depended_by", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: DependedByResult,
        expected: crate::test_utils::load_output_fixture("depended_by", "empty.toon"),
        format: Toon,
    }
}
