//! Output formatting for types command results.

use super::execute::TypesResult;
use crate::output::Outputable;

impl Outputable for TypesResult {
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

        if !self.types.is_empty() {
            lines.push(format!("Found {} type(s):", self.types.len()));
            for type_def in &self.types {
                // Show type signature with params
                let params_str = if type_def.params.is_empty() {
                    String::new()
                } else {
                    format!("({})", type_def.params)
                };
                let sig = format!("{}.{}{}", type_def.module, type_def.name, params_str);
                lines.push(format!("  {} [{}] line {}", sig, type_def.kind, type_def.line));

                // Show the definition if available
                if !type_def.definition.is_empty() {
                    lines.push(format!("       {}", type_def.definition));
                }
            }
        } else {
            lines.push("No types found.".to_string());
        }

        lines.join("\n")
    }
}
