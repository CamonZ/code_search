//! Output formatting tests for depends-on command.

#[cfg(test)]
mod tests {
    use super::super::execute::DependencyFunction;
    use db::types::{Call, FunctionRef, ModuleGroupResult, ModuleGroup};
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
    fn empty_result() -> ModuleGroupResult<DependencyFunction> {
        ModuleGroupResult {
            module_pattern: "MyApp.Controller".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<DependencyFunction> {
        ModuleGroupResult {
            module_pattern: "MyApp.Controller".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Service".to_string(),
                file: String::new(),
                entries: vec![DependencyFunction {
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
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<DependencyFunction> {
        ModuleGroupResult {
            module_pattern: "MyApp.Controller".to_string(),
            function_pattern: None,
            total_items: 2,
            items: vec![
                ModuleGroup {
                    name: "MyApp.Service".to_string(),
                    file: String::new(),
                    entries: vec![DependencyFunction {
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
                    function_count: None,
                },
                ModuleGroup {
                    name: "Phoenix.View".to_string(),
                    file: String::new(),
                    entries: vec![DependencyFunction {
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
                    function_count: None,
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
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: db::test_utils::load_output_fixture("depends_on", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: db::test_utils::load_output_fixture("depends_on", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<DependencyFunction>,
        expected: db::test_utils::load_output_fixture("depends_on", "empty.toon"),
        format: Toon,
    }
}
