//! Output formatting tests for reverse-trace command.

#[cfg(test)]
mod tests {
    use crate::types::{TraceDirection, TraceEntry, TraceResult};
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

MyApp.Service.fetch/1 [def] (service.ex:L10:20)";

    const MULTI_DEPTH_TABLE: &str = "\
Reverse trace to: MyApp.Repo.get
Max depth: 5

Found 2 caller(s) in chain:

MyApp.Service.fetch/1 [def] (service.ex:L10:20)
  ← @ L7 MyApp.Controller.index/1 [def] (controller.ex:L5:12)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            max_depth: 5,
            direction: TraceDirection::Backward,
            total_items: 0,
            entries: vec![],
        }
    }

    #[fixture]
    fn single_depth_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            max_depth: 5,
            direction: TraceDirection::Backward,
            total_items: 1,
            entries: vec![
                // Direct caller at depth 1
                TraceEntry {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 10,
                    end_line: 20,
                    file: "service.ex".to_string(),
                    depth: 1,
                    line: 15,
                    parent_index: None,
                },
            ],
        }
    }

    #[fixture]
    fn multi_depth_result() -> TraceResult {
        TraceResult {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            max_depth: 5,
            direction: TraceDirection::Backward,
            total_items: 2,
            entries: vec![
                TraceEntry {
                    module: "MyApp.Service".to_string(),
                    function: "fetch".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 10,
                    end_line: 20,
                    file: "service.ex".to_string(),
                    depth: 1,
                    line: 15,
                    parent_index: None,
                },
                TraceEntry {
                    module: "MyApp.Controller".to_string(),
                    function: "index".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    start_line: 5,
                    end_line: 12,
                    file: "controller.ex".to_string(),
                    depth: 2,
                    line: 7,
                    parent_index: Some(0),
                },
            ],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    #[rstest]
    fn test_empty_reverse_trace(empty_result: TraceResult) {
        use crate::output::Outputable;
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_single_depth_reverse_trace(single_depth_result: TraceResult) {
        use crate::output::Outputable;
        let output = single_depth_result.to_table();
        assert_eq!(output, SINGLE_TABLE);
    }

    #[rstest]
    fn test_multi_depth_reverse_trace(multi_depth_result: TraceResult) {
        use crate::output::Outputable;
        let output = multi_depth_result.to_table();
        assert_eq!(output, MULTI_DEPTH_TABLE);
    }
}
