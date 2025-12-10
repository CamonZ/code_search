//! Output formatting tests for depended-by command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DependedByResult, DependentCaller, DependentModule, DependentTarget};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Modules that depend on: MyApp.Repo

No dependents found.";

    const SINGLE_TABLE: &str = "\
Modules that depend on: MyApp.Repo

Found 1 call(s) from 1 module(s):

MyApp.Service:
  fetch/1 (lib/service.ex:10:20) [def]:
    → get/2 (L15)";

    const MULTIPLE_TABLE: &str = "\
Modules that depend on: MyApp.Repo

Found 2 call(s) from 2 module(s):

MyApp.Controller:
  show/1 (lib/controller.ex:15:25) [def]:
    → get/2 (L20)
MyApp.Service:
  fetch/1 (lib/service.ex:10:20) [def]:
    → get/2 (L15)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            total_calls: 0,
            modules: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            total_calls: 1,
            modules: vec![DependentModule {
                name: "MyApp.Service".to_string(),
                callers: vec![DependentCaller {
                    function: "fetch".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 10,
                    end_line: 20,
                    file: "lib/service.ex".to_string(),
                    targets: vec![DependentTarget {
                        function: "get".to_string(),
                        arity: 2,
                        line: 15,
                    }],
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            total_calls: 2,
            modules: vec![
                DependentModule {
                    name: "MyApp.Controller".to_string(),
                    callers: vec![DependentCaller {
                        function: "show".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 15,
                        end_line: 25,
                        file: "lib/controller.ex".to_string(),
                        targets: vec![DependentTarget {
                            function: "get".to_string(),
                            arity: 2,
                            line: 20,
                        }],
                    }],
                },
                DependentModule {
                    name: "MyApp.Service".to_string(),
                    callers: vec![DependentCaller {
                        function: "fetch".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 10,
                        end_line: 20,
                        file: "lib/service.ex".to_string(),
                        targets: vec![DependentTarget {
                            function: "get".to_string(),
                            arity: 2,
                            line: 15,
                        }],
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
