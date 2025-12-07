//! Output formatting for calls-from command results.

use crate::output::Outputable;
use super::execute::CallsFromResult;

impl Outputable for CallsFromResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = if self.function_pattern.is_empty() {
            format!("Calls from: {}", self.module_pattern)
        } else {
            format!("Calls from: {}.{}", self.module_pattern, self.function_pattern)
        };
        lines.push(header);
        lines.push(String::new());

        if !self.calls.is_empty() {
            lines.push(format!("Found {} call(s):", self.calls.len()));
            for call in &self.calls {
                let caller = format!("{}.{}", call.caller_module, call.caller_function);
                let callee = format!("{}.{}/{}", call.callee_module, call.callee_function, call.callee_arity);
                lines.push(format!(
                    "  [{}] {} ({}:{}) -> {}",
                    call.project, caller, call.file, call.line, callee
                ));
            }
        } else {
            lines.push("No calls found.".to_string());
        }

        lines.join("\n")
    }

    fn to_terse(&self) -> String {
        if self.calls.is_empty() {
            String::new()
        } else {
            self.calls
                .iter()
                .map(|c| {
                    format!(
                        "{},{},{},{},{},{},{},{},{}",
                        c.project, c.caller_module, c.caller_function,
                        c.callee_module, c.callee_function, c.callee_arity,
                        c.file, c.line, c.call_type
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
    use super::super::execute::CallEdge;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            calls: vec![],
        }
    }

    #[fixture]
    fn single_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
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
    fn multiple_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
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
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "list_users".to_string(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "all".to_string(),
                    callee_arity: 1,
                    file: "lib/my_app/accounts.ex".to_string(),
                    line: 22,
                    call_type: "remote".to_string(),
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: CallsFromResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Calls from: MyApp.Accounts.get_user"));
        assert!(output.contains("No calls found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: CallsFromResult) {
        let output = single_result.to_table();
        assert!(output.contains("Calls from: MyApp.Accounts.get_user"));
        assert!(output.contains("Found 1 call(s):"));
        assert!(output.contains("[default] MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: CallsFromResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Calls from: MyApp.Accounts"));
        assert!(output.contains("Found 2 call(s):"));
    }

    #[rstest]
    fn test_to_terse_empty(empty_result: CallsFromResult) {
        assert_eq!(empty_result.to_terse(), "");
    }

    #[rstest]
    fn test_to_terse_single(single_result: CallsFromResult) {
        let output = single_result.to_terse();
        assert_eq!(
            output,
            "default,MyApp.Accounts,get_user,MyApp.Repo,get,2,lib/my_app/accounts.ex,12,remote"
        );
    }

    #[rstest]
    fn test_format_json(single_result: CallsFromResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Accounts");
        assert_eq!(parsed["calls"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: CallsFromResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp.Accounts"));
        assert!(output.contains("function_pattern: get_user"));
    }

    #[rstest]
    fn test_format_toon_calls_fields(single_result: CallsFromResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("calls[1]{"));
        assert!(output.contains("caller_module"));
        assert!(output.contains("callee_module"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: CallsFromResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("calls[0]"));
    }
}
