//! Output formatting for specs command results.

use super::execute::SpecEntry;
use crate::output::Outputable;
use crate::types::ModuleCollectionResult;

impl Outputable for ModuleCollectionResult<SpecEntry> {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Build header
        let mut header = format!("Specs: {}", self.module_pattern);
        if let Some(ref func) = self.function_pattern {
            header.push_str(&format!(".{}", func));
        }
        if let Some(ref kind) = self.kind_filter {
            header.push_str(&format!(" (kind: {})", kind));
        }
        lines.push(header);
        lines.push(String::new());

        if !self.items.is_empty() {
            lines.push(format!(
                "Found {} spec(s) in {} module(s):",
                self.total_items,
                self.items.len()
            ));
            lines.push(String::new());

            for module in &self.items {
                lines.push(format!("{}:", module.name));
                for spec in &module.entries {
                    lines.push(format!(
                        "  {}/{} [{}] L{}",
                        spec.name, spec.arity, spec.kind, spec.line
                    ));

                    // Show the full spec if available, otherwise show inputs/returns
                    if !spec.full.is_empty() {
                        lines.push(format!("    {}", spec.full));
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
                        lines.push(format!("    {} :: {}", inputs, returns));
                    }
                }
            }
        } else {
            lines.push("No specs found.".to_string());
        }

        lines.join("\n")
    }
}
