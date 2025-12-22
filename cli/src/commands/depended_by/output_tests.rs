//! Output formatting tests for depended-by command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DependentCaller, DependentTarget};
    use crate::types::{ModuleGroupResult, ModuleGroup};
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
  fetch/1 [def] (service.ex:L10:20):
    → @ L15 get/2";

    const MULTIPLE_TABLE: &str = "\
Modules that depend on: MyApp.Repo

Found 2 call(s) from 2 module(s):

MyApp.Controller:
  show/1 [def] (controller.ex:L15:25):
    → @ L20 get/2
MyApp.Service:
  fetch/1 [def] (service.ex:L10:20):
    → @ L15 get/2";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleGroupResult<DependentCaller> {
        ModuleGroupResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: None,
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<DependentCaller> {
        ModuleGroupResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Service".to_string(),
                file: String::new(),
                entries: vec![DependentCaller {
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
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<DependentCaller> {
        ModuleGroupResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: None,
            total_items: 2,
            items: vec![
                ModuleGroup {
                    name: "MyApp.Controller".to_string(),
                    file: String::new(),
                    entries: vec![DependentCaller {
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
                    function_count: None,
                },
                ModuleGroup {
                    name: "MyApp.Service".to_string(),
                    file: String::new(),
                    entries: vec![DependentCaller {
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
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: crate::test_utils::load_output_fixture("depended_by", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: crate::test_utils::load_output_fixture("depended_by", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<DependentCaller>,
        expected: crate::test_utils::load_output_fixture("depended_by", "empty.toon"),
        format: Toon,
    }
}
