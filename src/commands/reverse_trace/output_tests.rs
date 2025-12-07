//! Output formatting tests for reverse-trace command.

#[cfg(test)]
mod tests {
    use super::super::execute::{ReverseTraceResult, ReverseTraceStep};
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
  [1] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2";

    const MULTI_DEPTH_TABLE: &str = "\
Reverse trace to: MyApp.Repo.get
Max depth: 5

Found 2 caller(s) in chain:
  [1] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2
    [2] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            steps: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            steps: vec![ReverseTraceStep {
                depth: 1,
                caller_module: "MyApp.Service".to_string(),
                caller_function: "fetch".to_string(),
                callee_module: "MyApp.Repo".to_string(),
                callee_function: "get".to_string(),
                callee_arity: 2,
                file: "lib/service.ex".to_string(),
                line: 15,
            }],
        }
    }

    #[fixture]
    fn multi_depth_result() -> ReverseTraceResult {
        ReverseTraceResult {
            target_module: "MyApp.Repo".to_string(),
            target_function: "get".to_string(),
            max_depth: 5,
            steps: vec![
                ReverseTraceStep {
                    depth: 1,
                    caller_module: "MyApp.Service".to_string(),
                    caller_function: "fetch".to_string(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/service.ex".to_string(),
                    line: 15,
                },
                ReverseTraceStep {
                    depth: 2,
                    caller_module: "MyApp.Controller".to_string(),
                    caller_function: "index".to_string(),
                    callee_module: "MyApp.Service".to_string(),
                    callee_function: "fetch".to_string(),
                    callee_arity: 1,
                    file: "lib/controller.ex".to_string(),
                    line: 7,
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
