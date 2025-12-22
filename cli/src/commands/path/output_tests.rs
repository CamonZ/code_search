//! Output formatting tests for path command.

#[cfg(test)]
mod tests {
    use super::super::execute::PathResult;
    use db::queries::path::{CallPath, PathStep};
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
        expected: db::test_utils::load_output_fixture("path", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_path_result,
        fixture_type: PathResult,
        expected: db::test_utils::load_output_fixture("path", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: PathResult,
        expected: db::test_utils::load_output_fixture("path", "empty.toon"),
        format: Toon,
    }
}
