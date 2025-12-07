//! Output formatting tests for trace command.

#[cfg(test)]
mod tests {
    use super::super::execute::{TraceResult, TraceStep};
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

    const SINGLE_JSON: &str = r#"{
  "start_module": "MyApp.Controller",
  "start_function": "index",
  "max_depth": 5,
  "steps": [
    {
      "depth": 1,
      "caller_module": "MyApp.Controller",
      "caller_function": "index",
      "callee_module": "MyApp.Service",
      "callee_function": "fetch",
      "callee_arity": 1,
      "file": "lib/controller.ex",
      "line": 7
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
max_depth: 5
start_function: index
start_module: MyApp.Controller
steps[1]{callee_arity,callee_function,callee_module,caller_function,caller_module,depth,file,line}:
  1,fetch,MyApp.Service,index,MyApp.Controller,1,lib/controller.ex,7";

    const EMPTY_TOON: &str = "\
max_depth: 5
start_function: index
start_module: MyApp.Controller
steps[0]:";

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
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: TraceResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: TraceResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
