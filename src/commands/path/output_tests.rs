//! Output formatting tests for path command.

#[cfg(test)]
mod tests {
    use super::super::execute::{CallPath, PathResult, PathStep};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Path from: MyApp.Controller.index to: MyApp.Repo.get
Max depth: 10

No path found.";

    const SINGLE_PATH_TABLE: &str = "\
Path from: MyApp.Controller.index to: MyApp.Repo.get
Max depth: 10

Found 1 path(s):

Path 1:
  [1] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1
    [2] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2";

    const SINGLE_PATH_JSON: &str = r#"{
  "from_module": "MyApp.Controller",
  "from_function": "index",
  "to_module": "MyApp.Repo",
  "to_function": "get",
  "max_depth": 10,
  "paths": [
    {
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
        },
        {
          "depth": 2,
          "caller_module": "MyApp.Service",
          "caller_function": "fetch",
          "callee_module": "MyApp.Repo",
          "callee_function": "get",
          "callee_arity": 2,
          "file": "lib/service.ex",
          "line": 15
        }
      ]
    }
  ]
}"#;

    const SINGLE_PATH_TOON: &str = "\
from_function: index
from_module: MyApp.Controller
max_depth: 10
paths[1]:
  - steps[2]{callee_arity,callee_function,callee_module,caller_function,caller_module,depth,file,line}:
    1,fetch,MyApp.Service,index,MyApp.Controller,1,lib/controller.ex,7
    2,get,MyApp.Repo,fetch,MyApp.Service,2,lib/service.ex,15
to_function: get
to_module: MyApp.Repo";

    const EMPTY_TOON: &str = "\
from_function: index
from_module: MyApp.Controller
max_depth: 10
paths[0]:
to_function: get
to_module: MyApp.Repo";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> PathResult {
        PathResult {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            max_depth: 10,
            paths: vec![],
        }
    }

    #[fixture]
    fn single_path_result() -> PathResult {
        PathResult {
            from_module: "MyApp.Controller".to_string(),
            from_function: "index".to_string(),
            to_module: "MyApp.Repo".to_string(),
            to_function: "get".to_string(),
            max_depth: 10,
            paths: vec![CallPath {
                steps: vec![
                    PathStep {
                        depth: 1,
                        caller_module: "MyApp.Controller".to_string(),
                        caller_function: "index".to_string(),
                        callee_module: "MyApp.Service".to_string(),
                        callee_function: "fetch".to_string(),
                        callee_arity: 1,
                        file: "lib/controller.ex".to_string(),
                        line: 7,
                    },
                    PathStep {
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
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: PathResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single_path,
        fixture: single_path_result,
        fixture_type: PathResult,
        expected: SINGLE_PATH_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_path_result,
        fixture_type: PathResult,
        expected: SINGLE_PATH_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_path_result,
        fixture_type: PathResult,
        expected: SINGLE_PATH_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: PathResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
