//! Output formatting for calls-from command results.

use crate::output::TableFormatter;
use crate::types::ModuleGroupResult;
use super::execute::CallerFunction;

impl TableFormatter for ModuleGroupResult<CallerFunction> {
    type Entry = CallerFunction;

    fn format_header(&self) -> String {
        if self.function_pattern.is_none() || self.function_pattern.as_ref().unwrap().is_empty() {
            format!("Calls from: {}", self.module_pattern)
        } else {
            format!("Calls from: {}.{}", self.module_pattern, self.function_pattern.as_ref().unwrap())
        }
    }

    fn format_empty_message(&self) -> String {
        "No calls found.".to_string()
    }

    fn format_summary(&self, total: usize, _module_count: usize) -> String {
        format!("Found {} call(s):", total)
    }

    fn format_module_header(&self, module_name: &str, module_file: &str) -> String {
        format!("{} ({})", module_name, module_file)
    }

    fn format_entry(&self, func: &CallerFunction, _module: &str, _file: &str) -> String {
        let kind_str = if func.kind.is_empty() {
            String::new()
        } else {
            format!(" [{}]", func.kind)
        };
        format!(
            "{}/{} ({}:{}){}",
            func.name, func.arity, func.start_line, func.end_line, kind_str
        )
    }

    fn format_entry_details(&self, func: &CallerFunction, module: &str, file: &str) -> Vec<String> {
        func.calls
            .iter()
            .map(|call| call.format_outgoing(module, file))
            .collect()
    }
}
