//! Output formatting for unused command results.

use crate::output::Outputable;
use super::execute::UnusedResult;

impl Outputable for UnusedResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let mut filters = Vec::new();
        if let Some(pattern) = &self.module_filter {
            filters.push(format!("module: {}", pattern));
        }
        if self.private_only {
            filters.push("private only".to_string());
        }
        if self.public_only {
            filters.push("public only".to_string());
        }
        if self.exclude_generated {
            filters.push("excluding generated".to_string());
        }

        let filter_info = if filters.is_empty() {
            String::new()
        } else {
            format!(" ({})", filters.join(", "))
        };

        lines.push(format!("Unused functions in project '{}'{}", self.project, filter_info));
        lines.push(String::new());

        if !self.functions.is_empty() {
            lines.push(format!("Found {} unused function(s):", self.functions.len()));
            for func in &self.functions {
                let sig = format!("{}.{}/{}", func.module, func.name, func.arity);
                lines.push(format!("  [{}] {}", func.kind, sig));
                lines.push(format!("       {}:{}", func.file, func.line));
            }
        } else {
            lines.push("No unused functions found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::UnusedFunction;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "\
Unused functions in project 'test_project'

No unused functions found.";

    const SINGLE_TABLE_OUTPUT: &str = "\
Unused functions in project 'test_project'

Found 1 unused function(s):
  [defp] MyApp.Accounts.unused_helper/0
       lib/accounts.ex:35";

    const FILTERED_TABLE_OUTPUT: &str = "\
Unused functions in project 'test_project' (module: Accounts)

Found 1 unused function(s):
  [defp] MyApp.Accounts.unused_helper/0
       lib/accounts.ex:35";

    #[fixture]
    fn empty_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![],
        }
    }

    #[fixture]
    fn single_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: None,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![UnusedFunction {
                module: "MyApp.Accounts".to_string(),
                name: "unused_helper".to_string(),
                arity: 0,
                kind: "defp".to_string(),
                file: "lib/accounts.ex".to_string(),
                line: 35,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> UnusedResult {
        UnusedResult {
            project: "test_project".to_string(),
            module_filter: Some("Accounts".to_string()),
            private_only: false,
            public_only: false,
            exclude_generated: false,
            functions: vec![UnusedFunction {
                module: "MyApp.Accounts".to_string(),
                name: "unused_helper".to_string(),
                arity: 0,
                kind: "defp".to_string(),
                file: "lib/accounts.ex".to_string(),
                line: 35,
            }],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: UnusedResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_single(single_result: UnusedResult) {
        assert_eq!(single_result.to_table(), SINGLE_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_filtered(filtered_result: UnusedResult) {
        assert_eq!(filtered_result.to_table(), FILTERED_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_format_json(single_result: UnusedResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["project"], "test_project");
        assert_eq!(parsed["functions"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["functions"][0]["name"], "unused_helper");
    }

    #[rstest]
    fn test_format_toon(single_result: UnusedResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
    }

    #[rstest]
    fn test_format_toon_function_fields(single_result: UnusedResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("functions[1]{"));
        assert!(output.contains("arity"));
        assert!(output.contains("file"));
        assert!(output.contains("line"));
        assert!(output.contains("module"));
        assert!(output.contains("name"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: UnusedResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
        assert!(output.contains("functions[0]"));
    }
}
