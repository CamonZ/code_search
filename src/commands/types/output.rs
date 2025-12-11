//! Output formatting for types command results.

use super::execute::TypeEntry;
use crate::output::Outputable;
use crate::types::ModuleCollectionResult;

impl Outputable for ModuleCollectionResult<TypeEntry> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Build header
        let mut header = format!("Types: {}", self.module_pattern);
        if let Some(ref name) = self.name_filter {
            header.push_str(&format!(".{}", name));
        }
        if let Some(ref kind) = self.kind_filter {
            header.push_str(&format!(" (kind: {})", kind));
        }
        lines.push(header);
        lines.push(String::new());

        if self.items.is_empty() {
            lines.push("No types found.".to_string());
        } else {
            lines.push(format!(
                "Found {} type(s) in {} module(s):",
                self.total_items,
                self.items.len()
            ));

            for module in &self.items {
                lines.push(String::new());
                lines.push(format!("{}:", module.name));

                for type_entry in &module.entries {
                    // Show type signature with params
                    let params_str = if type_entry.params.is_empty() {
                        String::new()
                    } else {
                        format!("({})", type_entry.params)
                    };
                    lines.push(format!(
                        "  {}{} [{}] L{}",
                        type_entry.name, params_str, type_entry.kind, type_entry.line
                    ));

                    // Show the definition if available
                    if !type_entry.definition.is_empty() {
                        lines.push(format!("    {}", type_entry.definition));
                    }
                }
            }
        }

        lines.join("\n")
    }
}
