//! Output formatting for calls-to command results.

use crate::output::TableFormatter;
use db::types::ModuleGroupResult;
use super::execute::CalleeFunction;

impl TableFormatter for ModuleGroupResult<CalleeFunction> {
    type Entry = CalleeFunction;

    fn format_header(&self) -> String {
        if self.function_pattern.is_none() || self.function_pattern.as_ref().unwrap().is_empty() {
            format!("Calls to: {}", self.module_pattern)
        } else {
            format!("Calls to: {}.{}", self.module_pattern, self.function_pattern.as_ref().unwrap())
        }
    }

    fn format_empty_message(&self) -> String {
        "No callers found.".to_string()
    }

    fn format_summary(&self, total: usize, _module_count: usize) -> String {
        format!("Found {} caller(s):", total)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        module_name.to_string()
    }

    fn format_entry(&self, func: &CalleeFunction, _module: &str, _file: &str) -> String {
        format!("{}/{}", func.name, func.arity)
    }

    fn format_entry_details(&self, func: &CalleeFunction, module: &str, _file: &str) -> Vec<String> {
        // Use empty context file since callers come from different files
        func.callers
            .iter()
            .map(|call| call.format_incoming(module, ""))
            .collect()
    }
}
