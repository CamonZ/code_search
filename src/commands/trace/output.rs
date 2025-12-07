//! Output formatting for trace command results.

use crate::output::Outputable;
use super::execute::TraceResult;

impl Outputable for TraceResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Trace from: {}.{}", self.start_module, self.start_function);
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if !self.steps.is_empty() {
            lines.push(format!("Found {} call(s) in chain:", self.steps.len()));
            for step in &self.steps {
                let indent = "  ".repeat(step.depth as usize);
                let caller = format!("{}.{}", step.caller_module, step.caller_function);
                let callee = format!("{}.{}/{}", step.callee_module, step.callee_function, step.callee_arity);
                lines.push(format!(
                    "{}[{}] {} ({}:{}) -> {}",
                    indent, step.depth, caller, step.file, step.line, callee
                ));
            }
        } else {
            lines.push("No calls found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::TraceStep;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

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

    #[rstest]
    fn test_to_table_empty(empty_result: TraceResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Trace from: MyApp.Controller.index"));
        assert!(output.contains("Max depth: 5"));
        assert!(output.contains("No calls found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: TraceResult) {
        let output = single_result.to_table();
        assert!(output.contains("Trace from: MyApp.Controller.index"));
        assert!(output.contains("Found 1 call(s) in chain:"));
        assert!(output.contains("[1] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1"));
    }

    #[rstest]
    fn test_to_table_multi_depth(multi_depth_result: TraceResult) {
        let output = multi_depth_result.to_table();
        assert!(output.contains("Found 2 call(s) in chain:"));
        assert!(output.contains("[1] MyApp.Controller.index"));
        assert!(output.contains("[2] MyApp.Service.fetch"));
    }

    #[rstest]
    fn test_format_json(single_result: TraceResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["start_module"], "MyApp.Controller");
        assert_eq!(parsed["start_function"], "index");
        assert_eq!(parsed["steps"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: TraceResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("start_module: MyApp.Controller"));
        assert!(output.contains("start_function: index"));
    }

    #[rstest]
    fn test_format_toon_steps(single_result: TraceResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("steps[1]"));
        assert!(output.contains("depth"));
        assert!(output.contains("caller_module"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: TraceResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("steps[0]"));
    }
}
