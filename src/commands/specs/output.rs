//! Output formatting for specs command results.

use super::execute::SpecsResult;
use crate::output::Outputable;

impl Outputable for SpecsResult {
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

        if !self.specs.is_empty() {
            lines.push(format!("Found {} spec(s):", self.specs.len()));
            for spec in &self.specs {
                // Show function signature
                let sig = format!("{}.{}/{}", spec.module, spec.name, spec.arity);
                lines.push(format!("  {} [{}] line {}", sig, spec.kind, spec.line));

                // Show the full spec if available, otherwise show inputs/returns
                if !spec.full.is_empty() {
                    lines.push(format!("       {}", spec.full));
                } else if !spec.inputs_string.is_empty() || !spec.return_string.is_empty() {
                    let inputs = if spec.inputs_string.is_empty() {
                        "()".to_string()
                    } else {
                        format!("({})", spec.inputs_string)
                    };
                    let returns = if spec.return_string.is_empty() {
                        "term()".to_string()
                    } else {
                        spec.return_string.clone()
                    };
                    lines.push(format!("       {} :: {}", inputs, returns));
                }
            }
        } else {
            lines.push("No specs found.".to_string());
        }

        lines.join("\n")
    }
}
