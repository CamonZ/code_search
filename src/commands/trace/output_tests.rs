//! Output formatting tests for trace command.

#[cfg(test)]
mod tests {
    use super::super::execute::{TraceCall, TraceNode, TraceResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Trace from: MyApp.Controller.index
Max depth: 5

No calls found.";

    const SINGLE_TABLE: &str = "\
Trace from: MyApp.Controller.index
Max depth: 5

Found 1 call(s) in chain:

MyApp.Controller.index/1 [def] (controller.ex:L5:12)
  → @ L7 MyApp.Service.fetch/1";

    const MULTI_DEPTH_TABLE: &str = "\
Trace from: MyApp.Controller.index
Max depth: 5

Found 2 call(s) in chain:

MyApp.Controller.index/1 [def] (controller.ex:L5:12)
  → @ L7 MyApp.Service.fetch/1 [def] (service.ex:L10:20)
    → @ L15 MyApp.Repo.get/2";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            total_calls: 0,
            roots: vec![],
        }
    }

    #[fixture]
    fn single_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            total_calls: 1,
            roots: vec![TraceNode {
                module: "MyApp.Controller".to_string(),
                function: "index".to_string(),
                arity: 1,
                kind: "def".to_string(),
                start_line: 5,
                end_line: 12,
                file: "lib/controller.ex".to_string(),
                calls: vec![TraceCall {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    line: 7,
                    children: vec![],
                }],
            }],
        }
    }

    #[fixture]
    fn multi_depth_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            total_calls: 2,
            roots: vec![TraceNode {
                module: "MyApp.Controller".to_string(),
                function: "index".to_string(),
                arity: 1,
                kind: "def".to_string(),
                start_line: 5,
                end_line: 12,
                file: "lib/controller.ex".to_string(),
                calls: vec![TraceCall {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    line: 7,
                    children: vec![TraceNode {
                        module: "MyApp.Service".to_string(),
                        function: "fetch".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 10,
                        end_line: 20,
                        file: "lib/service.ex".to_string(),
                        calls: vec![TraceCall {
                            module: "MyApp.Repo".to_string(),
                            function: "get".to_string(),
                            arity: 2,
                            line: 15,
                            children: vec![],
                        }],
                    }],
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
        fixture_type: TraceResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: TraceResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multi_depth,
        fixture: multi_depth_result,
        fixture_type: TraceResult,
        expected: MULTI_DEPTH_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: TraceResult,
        expected: crate::test_utils::load_output_fixture("trace", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: TraceResult,
        expected: crate::test_utils::load_output_fixture("trace", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: TraceResult,
        expected: crate::test_utils::load_output_fixture("trace", "empty.toon"),
        format: Toon,
    }
}
