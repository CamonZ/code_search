//! Output formatting for depended-by command results.

use crate::output::Outputable;
use super::execute::DependedByResult;

impl Outputable for DependedByResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Modules that depend on: {}", self.target_module));
        lines.push(String::new());

        if !self.dependents.is_empty() {
            lines.push(format!("Found {} module(s):", self.dependents.len()));
            for dep in &self.dependents {
                lines.push(format!("  {} ({} calls)", dep.module, dep.call_count));
            }
        } else {
            lines.push("No dependents found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::ModuleDependent;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![ModuleDependent {
                module: "MyApp.Service".to_string(),
                call_count: 3,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependedByResult {
        DependedByResult {
            target_module: "MyApp.Repo".to_string(),
            dependents: vec![
                ModuleDependent {
                    module: "MyApp.Service".to_string(),
                    call_count: 5,
                },
                ModuleDependent {
                    module: "MyApp.Controller".to_string(),
                    call_count: 2,
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: DependedByResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Modules that depend on: MyApp.Repo"));
        assert!(output.contains("No dependents found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: DependedByResult) {
        let output = single_result.to_table();
        assert!(output.contains("Modules that depend on: MyApp.Repo"));
        assert!(output.contains("Found 1 module(s):"));
        assert!(output.contains("MyApp.Service (3 calls)"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: DependedByResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Found 2 module(s):"));
        assert!(output.contains("MyApp.Service (5 calls)"));
        assert!(output.contains("MyApp.Controller (2 calls)"));
    }

    #[rstest]
    fn test_format_json(single_result: DependedByResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["target_module"], "MyApp.Repo");
        assert_eq!(parsed["dependents"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["dependents"][0]["module"], "MyApp.Service");
        assert_eq!(parsed["dependents"][0]["call_count"], 3);
    }

    #[rstest]
    fn test_format_toon(single_result: DependedByResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("target_module: MyApp.Repo"));
        assert!(output.contains("dependents[1]"));
    }
}
