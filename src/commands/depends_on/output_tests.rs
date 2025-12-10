//! Output formatting tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DependencyFunction, DependencyModule, DependsOnResult};
    use crate::types::{Call, FunctionRef};
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
    ← @ L7 MyApp.Controller.index/1 [def] (controller.ex:L5:12)";

    const MULTIPLE_TABLE: &str = "\
Dependencies of: MyApp.Controller

Found 2 call(s) to 2 module(s):

MyApp.Service:
  process/1:
    ← @ L7 MyApp.Controller.index/1 [def] (controller.ex:L5:12)
Phoenix.View:
  render/2:
    ← @ L20 MyApp.Controller.show/1 [def] (controller.ex:L15:25)";


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
                    callers: vec![Call {
                        caller: FunctionRef::with_definition(
                            "MyApp.Controller",
                            "index",
                            1,
                            "def",
                            "lib/controller.ex",
                            5,
                            12,
                        ),
                        callee: FunctionRef::new("MyApp.Service", "process", 1),
                        line: 7,
                        call_type: None,
                        depth: None,
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
                        callers: vec![Call {
                            caller: FunctionRef::with_definition(
                                "MyApp.Controller",
                                "index",
                                1,
                                "def",
                                "lib/controller.ex",
                                5,
                                12,
                            ),
                            callee: FunctionRef::new("MyApp.Service", "process", 1),
                            line: 7,
                            call_type: None,
                            depth: None,
                        }],
                    }],
                },
                DependencyModule {
                    name: "Phoenix.View".to_string(),
                    functions: vec![DependencyFunction {
                        name: "render".to_string(),
                        arity: 2,
                        callers: vec![Call {
                            caller: FunctionRef::with_definition(
                                "MyApp.Controller",
                                "show",
                                1,
                                "def",
                                "lib/controller.ex",
                                15,
                                25,
                            ),
                            callee: FunctionRef::new("Phoenix.View", "render", 2),
                            line: 20,
                            call_type: None,
                            depth: None,
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
