//! Output formatting for accepts command results.

use crate::output::TableFormatter;
use crate::types::ModuleGroupResult;
use super::execute::AcceptsInfo;

impl TableFormatter for ModuleGroupResult<AcceptsInfo> {
    type Entry = AcceptsInfo;

    fn format_header(&self) -> String {
        let pattern = self.function_pattern.as_ref().map(|s| s.as_str()).unwrap_or("*");
        format!("Functions accepting \"{}\"", pattern)
    }

    fn format_empty_message(&self) -> String {
        "No functions found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} function(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, accepts_info: &AcceptsInfo, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} ({}) → {}",
            accepts_info.name, accepts_info.arity, accepts_info.inputs, accepts_info.return_type
        )
    }
}
