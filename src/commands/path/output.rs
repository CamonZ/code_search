//! Output formatting for path command results.

use crate::output::Outputable;
use super::execute::PathResult;

impl Outputable for PathResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!(
            "Path from: {}.{} to: {}.{}",
            self.from_module, self.from_function, self.to_module, self.to_function
        );
        lines.push(header);
        lines.push(format!("Max depth: {}", self.max_depth));
        lines.push(String::new());

        if !self.paths.is_empty() {
            lines.push(format!("Found {} path(s):", self.paths.len()));
            for (i, path) in self.paths.iter().enumerate() {
                lines.push(String::new());
                lines.push(format!("Path {}:", i + 1));
                for step in &path.steps {
                    let indent = "  ".repeat(step.depth as usize);
                    let caller = format!("{}.{}", step.caller_module, step.caller_function);
                    let callee = format!("{}.{}/{}", step.callee_module, step.callee_function, step.callee_arity);
                    lines.push(format!(
                        "{}[{}] {} ({}:{}) -> {}",
                        indent, step.depth, caller, step.file, step.line, callee
                    ));
                }
            }
        } else {
            lines.push("No path found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::{CallPath, PathStep};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

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

    #[rstest]
    fn test_to_table_empty(empty_result: PathResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Path from: MyApp.Controller.index to: MyApp.Repo.get"));
        assert!(output.contains("Max depth: 10"));
        assert!(output.contains("No path found."));
    }

    #[rstest]
    fn test_to_table_single_path(single_path_result: PathResult) {
        let output = single_path_result.to_table();
        assert!(output.contains("Path from: MyApp.Controller.index to: MyApp.Repo.get"));
        assert!(output.contains("Found 1 path(s):"));
        assert!(output.contains("Path 1:"));
        assert!(output.contains("[1] MyApp.Controller.index (lib/controller.ex:7) -> MyApp.Service.fetch/1"));
        assert!(output.contains("[2] MyApp.Service.fetch (lib/service.ex:15) -> MyApp.Repo.get/2"));
    }

    #[rstest]
    fn test_format_json(single_path_result: PathResult) {
        let output = single_path_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["from_module"], "MyApp.Controller");
        assert_eq!(parsed["from_function"], "index");
        assert_eq!(parsed["to_module"], "MyApp.Repo");
        assert_eq!(parsed["to_function"], "get");
        assert_eq!(parsed["paths"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["paths"][0]["steps"].as_array().unwrap().len(), 2);
    }

    #[rstest]
    fn test_format_toon(single_path_result: PathResult) {
        let output = single_path_result.format(OutputFormat::Toon);
        assert!(output.contains("from_module: MyApp.Controller"));
        assert!(output.contains("from_function: index"));
        assert!(output.contains("to_module: MyApp.Repo"));
        assert!(output.contains("to_function: get"));
    }

    #[rstest]
    fn test_format_toon_paths(single_path_result: PathResult) {
        let output = single_path_result.format(OutputFormat::Toon);
        assert!(output.contains("paths[1]"));
        assert!(output.contains("steps[2]")); // 2 steps in the path
        assert!(output.contains("depth"));
        assert!(output.contains("caller_module"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: PathResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("paths[0]"));
    }
}
