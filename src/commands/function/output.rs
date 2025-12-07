//! Output formatting for function command results.

use crate::output::Outputable;
use super::execute::FunctionResult;

impl Outputable for FunctionResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Function: {}.{}", self.module_pattern, self.function_pattern);
        lines.push(header);
        lines.push(String::new());

        if !self.functions.is_empty() {
            lines.push(format!("Found {} signature(s):", self.functions.len()));
            for func in &self.functions {
                let signature = format!(
                    "{}.{}/{}",
                    func.module, func.name, func.arity
                );
                lines.push(format!("  [{}] {}", func.project, signature));
                if !func.args.is_empty() {
                    lines.push(format!("       args: {}", func.args));
                }
                if !func.return_type.is_empty() {
                    lines.push(format!("       returns: {}", func.return_type));
                }
            }
        } else {
            lines.push("No functions found.".to_string());
        }

        lines.join("\n")
    }

    fn to_terse(&self) -> String {
        if self.functions.is_empty() {
            String::new()
        } else {
            self.functions
                .iter()
                .map(|f| {
                    format!(
                        "{},{},{},{},{},{}",
                        f.project, f.module, f.name, f.arity, f.args, f.return_type
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::FunctionSignature;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![],
        }
    }

    #[fixture]
    fn single_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![FunctionSignature {
                project: "default".to_string(),
                module: "MyApp.Accounts".to_string(),
                name: "get_user".to_string(),
                arity: 1,
                args: "integer()".to_string(),
                return_type: "User.t() | nil".to_string(),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![
                FunctionSignature {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    args: "integer()".to_string(),
                    return_type: "User.t() | nil".to_string(),
                },
                FunctionSignature {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "get_user".to_string(),
                    arity: 2,
                    args: "integer(), keyword()".to_string(),
                    return_type: "User.t() | nil".to_string(),
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: FunctionResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Function: MyApp.Accounts.get_user"));
        assert!(output.contains("No functions found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: FunctionResult) {
        let output = single_result.to_table();
        assert!(output.contains("Function: MyApp.Accounts.get_user"));
        assert!(output.contains("Found 1 signature(s):"));
        assert!(output.contains("[default] MyApp.Accounts.get_user/1"));
        assert!(output.contains("args: integer()"));
        assert!(output.contains("returns: User.t() | nil"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: FunctionResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Found 2 signature(s):"));
        assert!(output.contains("get_user/1"));
        assert!(output.contains("get_user/2"));
    }

    #[rstest]
    fn test_to_terse_empty(empty_result: FunctionResult) {
        assert_eq!(empty_result.to_terse(), "");
    }

    #[rstest]
    fn test_to_terse_single(single_result: FunctionResult) {
        let output = single_result.to_terse();
        assert_eq!(
            output,
            "default,MyApp.Accounts,get_user,1,integer(),User.t() | nil"
        );
    }

    #[rstest]
    fn test_format_json(single_result: FunctionResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Accounts");
        assert_eq!(parsed["functions"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: FunctionResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp.Accounts"));
        assert!(output.contains("function_pattern: get_user"));
    }

    #[rstest]
    fn test_format_toon_function_fields(single_result: FunctionResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("functions[1]{"));
        assert!(output.contains("args"));
        assert!(output.contains("return_type"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: FunctionResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("functions[0]"));
    }
}
