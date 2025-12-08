//! Output formatting tests for trace command.

#[cfg(test)]
mod tests {
    use super::super::execute::TraceResult;
    use crate::queries::trace::TraceStep;
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
  [1] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1";

    const MULTI_DEPTH_TABLE: &str = "\
Trace from: MyApp.Controller.index
Max depth: 5

Found 2 call(s) in chain:
  [1] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1
    [2] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            steps: vec![],
        }
    }

    #[fixture]
    fn single_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            steps: vec![TraceStep {
                depth: 1,
                caller_module: "MyApp.Controller".to_string(),
                caller_function: "index".to_string(),
                callee_module: "MyApp.Service".to_string(),
                callee_function: "fetch".to_string(),
                callee_arity: 1,
                file: "lib/controller.ex".to_string(),
                line: 7,
            }],
        }
    }

    #[fixture]
    fn multi_depth_result() -> TraceResult {
        TraceResult {
            start_module: "MyApp.Controller".to_string(),
            start_function: "index".to_string(),
            max_depth: 5,
            steps: vec![
                TraceStep {
                    depth: 1,
                    caller_module: "MyApp.Controller".to_string(),
                    caller_function: "index".to_string(),
                    callee_module: "MyApp.Service".to_string(),
                    callee_function: "fetch".to_string(),
                    callee_arity: 1,
                    file: "lib/controller.ex".to_string(),
                    line: 7,
                },
                TraceStep {
                    depth: 2,
                    caller_module: "MyApp.Service".to_string(),
                    caller_function: "fetch".to_string(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/service.ex".to_string(),
                    line: 15,
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
