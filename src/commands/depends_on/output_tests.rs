//! Output formatting tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DependencyCaller, DependencyFunction, DependencyModule, DependsOnResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Dependencies of: MyApp.Controller

No dependencies found.";

    const SINGLE_TABLE: &str = "\
Dependencies of: MyApp.Controller

Found 1 call(s) to 1 module(s):

MyApp.Service:
  process/1:
    ← MyApp.Controller.index/1 (lib/controller.ex:5:12 L7) [def]";

    const MULTIPLE_TABLE: &str = "\
Dependencies of: MyApp.Controller

Found 2 call(s) to 2 module(s):

MyApp.Service:
  process/1:
    ← MyApp.Controller.index/1 (lib/controller.ex:5:12 L7) [def]
Phoenix.View:
  render/2:
    ← MyApp.Controller.show/1 (lib/controller.ex:15:25 L20) [def]";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            total_calls: 0,
            modules: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            total_calls: 1,
            modules: vec![DependencyModule {
                name: "MyApp.Service".to_string(),
                functions: vec![DependencyFunction {
                    name: "process".to_string(),
                    arity: 1,
                    callers: vec![DependencyCaller {
                        module: "MyApp.Controller".to_string(),
                        function: "index".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 5,
                        end_line: 12,
                        file: "lib/controller.ex".to_string(),
                        line: 7,
                    }],
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            total_calls: 2,
            modules: vec![
                DependencyModule {
                    name: "MyApp.Service".to_string(),
                    functions: vec![DependencyFunction {
                        name: "process".to_string(),
                        arity: 1,
                        callers: vec![DependencyCaller {
                            module: "MyApp.Controller".to_string(),
                            function: "index".to_string(),
                            arity: 1,
                            kind: "def".to_string(),
                            start_line: 5,
                            end_line: 12,
                            file: "lib/controller.ex".to_string(),
                            line: 7,
                        }],
                    }],
                },
                DependencyModule {
                    name: "Phoenix.View".to_string(),
                    functions: vec![DependencyFunction {
                        name: "render".to_string(),
                        arity: 2,
                        callers: vec![DependencyCaller {
                            module: "MyApp.Controller".to_string(),
                            function: "show".to_string(),
                            arity: 1,
                            kind: "def".to_string(),
                            start_line: 15,
                            end_line: 25,
                            file: "lib/controller.ex".to_string(),
                            line: 20,
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
