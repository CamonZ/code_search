//! Output formatting for depends-on command results.

use crate::output::TableFormatter;
use crate::types::ModuleGroupResult;
use super::execute::DependencyFunction;

impl TableFormatter for ModuleGroupResult<DependencyFunction> {
    type Entry = DependencyFunction;

    fn format_header(&self) -> String {
        format!("Dependencies of: {}", self.module_pattern)
    }

    fn format_empty_message(&self) -> String {
        "No dependencies found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} call(s) to {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, func: &DependencyFunction, _module: &str, _file: &str) -> String {
        format!("{}/{}:", func.name, func.arity)
    }

    fn format_entry_details(&self, func: &DependencyFunction, module: &str, _file: &str) -> Vec<String> {
        // Use empty context since callers come from different files
        func.callers
            .iter()
            .map(|call| call.format_incoming(module, ""))
            .collect()
    }
}
