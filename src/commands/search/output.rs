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
                lines.push(format!("  [{}] {}", m.project, m.name));
            }
        }

        if !self.functions.is_empty() {
            lines.push(format!("Functions ({}):", self.functions.len()));
            for f in &self.functions {
                let sig = if f.return_type.is_empty() {
                    format!("{}.{}/{}", f.module, f.name, f.arity)
                } else {
                    format!("{}.{}/{} -> {}", f.module, f.name, f.arity, f.return_type)
                };
                lines.push(format!("  [{}] {}", f.project, sig));
            }
        }

        if self.modules.is_empty() && self.functions.is_empty() {
            lines.push("No results found.".to_string());
        }

        lines.join("\n")
    }
}
