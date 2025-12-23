//! Output formatting for function command results.

use crate::output::TableFormatter;
use db::types::ModuleGroupResult;
use super::execute::FuncSig;

impl TableFormatter for ModuleGroupResult<FuncSig> {
    type Entry = FuncSig;

    fn format_header(&self) -> String {
        let function_pattern = self.function_pattern.as_deref().unwrap_or("*");
        format!("Function: {}.{}", self.module_pattern, function_pattern)
    }

    fn format_empty_message(&self) -> String {
        "No functions found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} signature(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, func: &FuncSig, _module: &str, _file: &str) -> String {
        format!("{}/{}", func.name, func.arity)
    }

    fn format_entry_details(&self, func: &FuncSig, _module: &str, _file: &str) -> Vec<String> {
        let mut details = Vec::new();
        if !func.args.is_empty() {
            details.push(format!("args: {}", func.args));
        }
        if !func.return_type.is_empty() {
            details.push(format!("returns: {}", func.return_type));
        }
        details
    }
}
