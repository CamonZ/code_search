//! Output formatting for reverse-trace command results.

use crate::output::Outputable;
use super::execute::ReverseTraceResult;

impl Outputable for ReverseTraceResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Reverse trace to: {}.{}", self.target_module, self.target_function);
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if !self.steps.is_empty() {
            lines.push(format!("Found {} caller(s) in chain:", self.steps.len()));
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
            lines.push("No callers found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::ReverseTraceStep;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

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

    #[rstest]
    fn test_to_table_empty(empty_result: ReverseTraceResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Reverse trace to: MyApp.Repo.get"));
        assert!(output.contains("Max depth: 5"));
        assert!(output.contains("No callers found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: ReverseTraceResult) {
        let output = single_result.to_table();
        assert!(output.contains("Reverse trace to: MyApp.Repo.get"));
        assert!(output.contains("Found 1 caller(s) in chain:"));
        assert!(output.contains("[1] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2"));
    }

    #[rstest]
    fn test_to_table_multi_depth(multi_depth_result: ReverseTraceResult) {
        let output = multi_depth_result.to_table();
        assert!(output.contains("Found 2 caller(s) in chain:"));
        assert!(output.contains("[1] MyApp.Service.fetch"));
        assert!(output.contains("[2] MyApp.Controller.index"));
    }

    #[rstest]
    fn test_format_json(single_result: ReverseTraceResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["target_module"], "MyApp.Repo");
        assert_eq!(parsed["target_function"], "get");
        assert_eq!(parsed["steps"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: ReverseTraceResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("target_module: MyApp.Repo"));
        assert!(output.contains("target_function: get"));
    }

    #[rstest]
    fn test_format_toon_steps(single_result: ReverseTraceResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("steps[1]"));
        assert!(output.contains("depth"));
        assert!(output.contains("caller_module"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: ReverseTraceResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("steps[0]"));
    }
}
