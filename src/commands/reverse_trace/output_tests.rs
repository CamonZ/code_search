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

    const SINGLE_JSON: &str = r#"{
  "target_module": "MyApp.Repo",
  "target_function": "get",
  "max_depth": 5,
  "steps": [
    {
      "depth": 1,
      "caller_module": "MyApp.Service",
      "caller_function": "fetch",
      "callee_module": "MyApp.Repo",
      "callee_function": "get",
      "callee_arity": 2,
      "file": "lib/service.ex",
      "line": 15
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
max_depth: 5
steps[1]{callee_arity,callee_function,callee_module,caller_function,caller_module,depth,file,line}:
  2,get,MyApp.Repo,fetch,MyApp.Service,1,lib/service.ex,15
target_function: get
target_module: MyApp.Repo";

    const EMPTY_TOON: &str = "\
max_depth: 5
steps[0]:
target_function: get
target_module: MyApp.Repo";

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
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ReverseTraceResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ReverseTraceResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
