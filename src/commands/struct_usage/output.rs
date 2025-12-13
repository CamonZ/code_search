//! Output formatting for struct-usage command results.

use crate::output::TableFormatter;
use crate::types::ModuleGroupResult;
use super::execute::UsageInfo;

impl TableFormatter for ModuleGroupResult<UsageInfo> {
    type Entry = UsageInfo;

    fn format_header(&self) -> String {
        let pattern = self.function_pattern.as_ref().map(|s| s.as_str()).unwrap_or("*");
        format!("Functions using \"{}\"", pattern)
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

    fn format_entry(&self, usage_info: &UsageInfo, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} accepts: {} returns: {}",
            usage_info.name, usage_info.arity, usage_info.inputs, usage_info.returns
        )
    }
}
