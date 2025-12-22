//! Output formatting for depended-by command results.

use crate::output::TableFormatter;
use db::types::ModuleGroupResult;
use super::execute::DependentCaller;

impl TableFormatter for ModuleGroupResult<DependentCaller> {
    type Entry = DependentCaller;

    fn format_header(&self) -> String {
        format!("Modules that depend on: {}", self.module_pattern)
    }

    fn format_empty_message(&self) -> String {
        "No dependents found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} call(s) from {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, caller: &DependentCaller, _module: &str, _file: &str) -> String {
        let kind_str = if caller.kind.is_empty() {
            String::new()
        } else {
            format!(" [{}]", caller.kind)
        };
        // Extract just the filename from path
        let filename = caller.file.rsplit('/').next().unwrap_or(&caller.file);
        format!(
            "{}/{}{} ({}:L{}:{}):",
            caller.function, caller.arity, kind_str,
            filename, caller.start_line, caller.end_line
        )
    }

    fn format_entry_details(&self, caller: &DependentCaller, _module: &str, _file: &str) -> Vec<String> {
        caller
            .targets
            .iter()
            .map(|target| format!("→ @ L{} {}/{}", target.line, target.function, target.arity))
            .collect()
    }
}
