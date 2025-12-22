//! Output formatting for search command results.

use crate::output::Outputable;
use super::execute::SearchResult;

impl Outputable for SearchResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Search: {} ({})", self.pattern, self.kind));
        lines.push(String::new());

        if !self.modules.is_empty() {
            lines.push(format!("Modules ({}):", self.modules.len()));
            for m in &self.modules {
                lines.push(format!("  {}", m.name));
            }
        }

        if !self.function_modules.is_empty() {
            let total = self.total_functions.unwrap_or(0);
            lines.push(format!(
                "Functions ({}) in {} module(s):",
                total,
                self.function_modules.len()
            ));
            lines.push(String::new());

            for module in &self.function_modules {
                lines.push(format!("{}:", module.name));
                for f in &module.functions {
                    let sig = if f.return_type.is_empty() {
                        format!("{}/{}", f.name, f.arity)
                    } else {
                        format!("{}/{} -> {}", f.name, f.arity, f.return_type)
                    };
                    lines.push(format!("  {}", sig));
                }
            }
        }

        if self.modules.is_empty() && self.function_modules.is_empty() {
            lines.push("No results found.".to_string());
        }

        lines.join("\n")
    }
}
