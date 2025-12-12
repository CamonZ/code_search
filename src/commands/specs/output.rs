//! Output formatting for specs command results.

use super::execute::SpecEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<SpecEntry> {
    type Entry = SpecEntry;

    fn format_header(&self) -> String {
        let mut header = format!("Specs: {}", self.module_pattern);
        if let Some(ref func) = self.function_pattern {
            header.push_str(&format!(".{}", func));
        }
        if let Some(ref kind) = self.kind_filter {
            header.push_str(&format!(" (kind: {})", kind));
        }
        header
    }

    fn format_empty_message(&self) -> String {
        "No specs found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} spec(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, spec: &SpecEntry, _module: &str, _file: &str) -> String {
        format!("{}/{} [{}] L{}", spec.name, spec.arity, spec.kind, spec.line)
    }

    fn format_entry_details(&self, spec: &SpecEntry, _module: &str, _file: &str) -> Vec<String> {
        // Show the full spec if available, otherwise show inputs/returns
        if !spec.full.is_empty() {
            vec![spec.full.clone()]
        } else if !spec.inputs.is_empty() || !spec.returns.is_empty() {
            let inputs = if spec.inputs.is_empty() {
                "()".to_string()
            } else {
                format!("({})", spec.inputs)
            };
            let returns = if spec.returns.is_empty() {
                "term()".to_string()
            } else {
                spec.returns.clone()
            };
            vec![format!("{} :: {}", inputs, returns)]
        } else {
            Vec::new()
        }
    }
}
