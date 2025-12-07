//! Output formatting for depends-on command results.

use crate::output::Outputable;
use super::execute::DependsOnResult;

impl Outputable for DependsOnResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Dependencies of: {}", self.source_module));
        lines.push(String::new());

        if !self.dependencies.is_empty() {
            lines.push(format!("Found {} module(s):", self.dependencies.len()));
            for dep in &self.dependencies {
                lines.push(format!("  {} ({} calls)", dep.module, dep.call_count));
            }
        } else {
            lines.push("No dependencies found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::ModuleDependency;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![],
        }
    }

    #[fixture]
    fn single_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![ModuleDependency {
                module: "MyApp.Service".to_string(),
                call_count: 5,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> DependsOnResult {
        DependsOnResult {
            source_module: "MyApp.Controller".to_string(),
            dependencies: vec![
                ModuleDependency {
                    module: "MyApp.Service".to_string(),
                    call_count: 5,
                },
                ModuleDependency {
                    module: "Phoenix.View".to_string(),
                    call_count: 2,
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: DependsOnResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Dependencies of: MyApp.Controller"));
        assert!(output.contains("No dependencies found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: DependsOnResult) {
        let output = single_result.to_table();
        assert!(output.contains("Dependencies of: MyApp.Controller"));
        assert!(output.contains("Found 1 module(s):"));
        assert!(output.contains("MyApp.Service (5 calls)"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: DependsOnResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Found 2 module(s):"));
        assert!(output.contains("MyApp.Service (5 calls)"));
        assert!(output.contains("Phoenix.View (2 calls)"));
    }

    #[rstest]
    fn test_format_json(single_result: DependsOnResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["source_module"], "MyApp.Controller");
        assert_eq!(parsed["dependencies"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["dependencies"][0]["module"], "MyApp.Service");
        assert_eq!(parsed["dependencies"][0]["call_count"], 5);
    }

    #[rstest]
    fn test_format_toon(single_result: DependsOnResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("source_module: MyApp.Controller"));
        assert!(output.contains("dependencies[1]"));
    }
}
