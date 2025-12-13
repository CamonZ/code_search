//! Output formatting for returns command results.

use crate::output::TableFormatter;
use crate::types::ModuleGroupResult;
use super::execute::ReturnInfo;

impl TableFormatter for ModuleGroupResult<ReturnInfo> {
    type Entry = ReturnInfo;

    fn format_header(&self) -> String {
        let pattern = self.function_pattern.as_ref().map(|s| s.as_str()).unwrap_or("*");
        format!("Functions returning \"{}\"", pattern)
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

    fn format_entry(&self, return_info: &ReturnInfo, _module: &str, _file: &str) -> String {
        format!("{}/{} → {}", return_info.name, return_info.arity, return_info.return_type)
    }
}
