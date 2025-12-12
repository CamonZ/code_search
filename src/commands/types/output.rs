//! Output formatting for types command results.

use super::execute::TypeEntry;
use crate::output::TableFormatter;
use crate::types::ModuleCollectionResult;

impl TableFormatter for ModuleCollectionResult<TypeEntry> {
    type Entry = TypeEntry;

    fn format_header(&self) -> String {
        let mut header = format!("Types: {}", self.module_pattern);
        if let Some(ref name) = self.name_filter {
            header.push_str(&format!(".{}", name));
        }
        if let Some(ref kind) = self.kind_filter {
            header.push_str(&format!(" (kind: {})", kind));
        }
        header
    }

    fn format_empty_message(&self) -> String {
        "No types found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} type(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, type_entry: &TypeEntry, _module: &str, _file: &str) -> String {
        let params_str = if type_entry.params.is_empty() {
            String::new()
        } else {
            format!("({})", type_entry.params)
        };
        format!(
            "{}{} [{}] L{}",
            type_entry.name, params_str, type_entry.kind, type_entry.line
        )
    }

    fn format_entry_details(&self, type_entry: &TypeEntry, _module: &str, _file: &str) -> Vec<String> {
        if !type_entry.definition.is_empty() {
            vec![type_entry.definition.clone()]
        } else {
            Vec::new()
        }
    }

    fn blank_before_module(&self) -> bool {
        true
    }

    fn blank_after_summary(&self) -> bool {
        false
    }
}
