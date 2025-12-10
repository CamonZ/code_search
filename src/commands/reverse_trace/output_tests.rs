//! Output formatting tests for reverse-trace command.

#[cfg(test)]
mod tests {
    use super::super::execute::{ReverseTraceNode, ReverseTraceResult, ReverseTraceTarget};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Reverse trace to: MyApp.Repo.get
Max depth: 5

No callers found.";

    const SINGLE_TABLE: &str = "\
Reverse trace to: MyApp.Repo.get
Max depth: 5

Found 1 caller(s) in chain:

MyApp.Service.fetch/1 (lib/service.ex:10:20) [def]
  → MyApp.Repo.get/2 (L15)";

    const MULTI_DEPTH_TABLE: &str = "\
Reverse trace to: MyApp.Repo.get
Max depth: 5

Found 2 caller(s) in chain:

MyApp.Service.fetch/1 (lib/service.ex:10:20) [def]
  → MyApp.Repo.get/2 (L15)
  MyApp.Controller.index/1 (lib/controller.ex:5:12) [def]
    → MyApp.Service.fetch/1 (L7)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            total_callers: 0,
            roots: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            total_callers: 1,
            roots: vec![ReverseTraceNode {
                module: "MyApp.Service".to_string(),
                function: "fetch".to_string(),
                arity: 1,
                kind: "def".to_string(),
                start_line: 10,
                end_line: 20,
                file: "lib/service.ex".to_string(),
                targets: vec![ReverseTraceTarget {
                    module: "MyApp.Repo".to_string(),
                    function: "get".to_string(),
                    arity: 2,
                    line: 15,
                }],
                callers: vec![],
            }],
        }
    }

    #[fixture]
    fn multi_depth_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            total_callers: 2,
            roots: vec![ReverseTraceNode {
                module: "MyApp.Service".to_string(),
                function: "fetch".to_string(),
                arity: 1,
                kind: "def".to_string(),
                start_line: 10,
                end_line: 20,
                file: "lib/service.ex".to_string(),
                targets: vec![ReverseTraceTarget {
                    module: "MyApp.Repo".to_string(),
                    function: "get".to_string(),
                    arity: 2,
                    line: 15,
                }],
                callers: vec![ReverseTraceNode {
                    module: "MyApp.Controller".to_string(),
                    function: "index".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 5,
                    end_line: 12,
                    file: "lib/controller.ex".to_string(),
                    targets: vec![ReverseTraceTarget {
                        module: "MyApp.Service".to_string(),
                        function: "fetch".to_string(),
                        arity: 1,
                        line: 7,
                    }],
                    callers: vec![],
                }],
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ReverseTraceResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ReverseTraceResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multi_depth,
        fixture: multi_depth_result,
        fixture_type: ReverseTraceResult,
        expected: MULTI_DEPTH_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ReverseTraceResult,
        expected: crate::test_utils::load_output_fixture("reverse_trace", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ReverseTraceResult,
        expected: crate::test_utils::load_output_fixture("reverse_trace", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ReverseTraceResult,
        expected: crate::test_utils::load_output_fixture("reverse_trace", "empty.toon"),
        format: Toon,
    }
}
