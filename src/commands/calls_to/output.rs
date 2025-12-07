//! Output formatting for calls-to command results.

use crate::output::Outputable;
use super::execute::CallsToResult;

impl Outputable for CallsToResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = if self.function_pattern.is_empty() {
            format!("Calls to: {}", self.module_pattern)
        } else {
            format!("Calls to: {}.{}", self.module_pattern, self.function_pattern)
        };
        lines.push(header);
        lines.push(String::new());

        if !self.calls.is_empty() {
            lines.push(format!("Found {} caller(s):", self.calls.len()));
            for call in &self.calls {
                let caller = format!("{}.{}", call.caller_module, call.caller_function);
                let callee = format!("{}.{}/{}", call.callee_module, call.callee_function, call.callee_arity);
                lines.push(format!(
                    "  [{}] {} ({}:{}) -> {}",
                    call.project, caller, call.file, call.line, callee
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
    use super::super::execute::CallEdge;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: "get".to_string(),
            calls: vec![],
        }
    }

    #[fixture]
    fn single_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: "get".to_string(),
            calls: vec![CallEdge {
                project: "default".to_string(),
                caller_module: "MyApp.Accounts".to_string(),
                caller_function: "get_user".to_string(),
                callee_module: "MyApp.Repo".to_string(),
                callee_function: "get".to_string(),
                callee_arity: 2,
                file: "lib/my_app/accounts.ex".to_string(),
                line: 12,
                call_type: "remote".to_string(),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: String::new(),
            calls: vec![
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "get_user".to_string(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/my_app/accounts.ex".to_string(),
                    line: 12,
                    call_type: "remote".to_string(),
                },
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Users".to_string(),
                    caller_function: "update_user".to_string(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/my_app/users.ex".to_string(),
                    line: 40,
                    call_type: "remote".to_string(),
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: CallsToResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Calls to: MyApp.Repo.get"));
        assert!(output.contains("No callers found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: CallsToResult) {
        let output = single_result.to_table();
        assert!(output.contains("Calls to: MyApp.Repo.get"));
        assert!(output.contains("Found 1 caller(s):"));
        assert!(output.contains("[default] MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: CallsToResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Calls to: MyApp.Repo"));
        assert!(output.contains("Found 2 caller(s):"));
    }

    #[rstest]
    fn test_format_json(single_result: CallsToResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Repo");
        assert_eq!(parsed["calls"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: CallsToResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp.Repo"));
        assert!(output.contains("function_pattern: get"));
    }

    #[rstest]
    fn test_format_toon_calls_fields(single_result: CallsToResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("calls[1]{"));
        assert!(output.contains("caller_module"));
        assert!(output.contains("callee_module"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: CallsToResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("calls[0]"));
    }
}
