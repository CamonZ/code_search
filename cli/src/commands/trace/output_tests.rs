//! Output formatting tests for trace command.

#[cfg(test)]
mod tests {
    use db::types::{TraceDirection, TraceEntry, TraceResult};
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
  → @ L7 MyApp.Service.fetch/1 [def] (service.ex:L10:20)";

    const MULTI_DEPTH_TABLE: &str = "\
Trace from: MyApp.Controller.index
Max depth: 5

Found 2 call(s) in chain:

MyApp.Controller.index/1 [def] (controller.ex:L5:12)
  → @ L7 MyApp.Service.fetch/1 [def] (service.ex:L10:20)
    → @ L15 MyApp.Repo.get/2 (repo.ex:L30:40)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            max_depth: 5,
            direction: TraceDirection::Forward,
            total_items: 0,
            entries: vec![],
        }
    }

    #[fixture]
    fn single_depth_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            max_depth: 5,
            direction: TraceDirection::Forward,
            total_items: 1,
            entries: vec![
                // Root entry: the starting function
                TraceEntry {
                    module: "MyApp.Controller".to_string(),
                    function: "index".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 5,
                    end_line: 12,
                    file: "/path/to/controller.ex".to_string(),
                    depth: 0,
                    line: 0,
                    parent_index: None,
                },
                // Callee at depth 1
                TraceEntry {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 10,
                    end_line: 20,
                    file: "/path/to/service.ex".to_string(),
                    depth: 1,
                    line: 7,
                    parent_index: Some(0),
                },
            ],
        }
    }

    #[fixture]
    fn multi_depth_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Controller".to_string(),
            function: "index".to_string(),
            max_depth: 5,
            direction: TraceDirection::Forward,
            total_items: 2,
            entries: vec![
                TraceEntry {
                    module: "MyApp.Controller".to_string(),
                    function: "index".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 5,
                    end_line: 12,
                    file: "/path/to/controller.ex".to_string(),
                    depth: 0,
                    line: 0,
                    parent_index: None,
                },
                TraceEntry {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 10,
                    end_line: 20,
                    file: "/path/to/service.ex".to_string(),
                    depth: 1,
                    line: 7,
                    parent_index: Some(0),
                },
                TraceEntry {
                    module: "MyApp.Repo".to_string(),
                    function: "get".to_string(),
                    arity: 2,
                    kind: String::new(),
                    start_line: 30,
                    end_line: 40,
                    file: "repo.ex".to_string(),
                    depth: 2,
                    line: 15,
                    parent_index: Some(1),
                },
            ],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    #[rstest]
    fn test_empty_trace(empty_result: TraceResult) {
        use crate::output::Outputable;
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_single_depth_trace(single_depth_result: TraceResult) {
        use crate::output::Outputable;
        let output = single_depth_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_multi_depth_trace(multi_depth_result: TraceResult) {
        use crate::output::Outputable;
        let output = multi_depth_result.to_table();
        assert_eq!(output, MULTI_DEPTH_TABLE);
    }
}
