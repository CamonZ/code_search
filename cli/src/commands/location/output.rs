//! Output formatting for location command results.

use crate::output::Outputable;
use super::execute::LocationResult;

impl Outputable for LocationResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Location: {}.{}", self.module_pattern, self.function_pattern));
        lines.push(String::new());

        if !self.modules.is_empty() {
            let func_count: usize = self.modules.iter().map(|m| m.functions.len()).sum();
            lines.push(format!(
                "Found {} clause(s) in {} function(s) across {} module(s):",
                self.total_clauses,
                func_count,
                self.modules.len()
            ));
            lines.push(String::new());

            for module in &self.modules {
                lines.push(format!("{}:", module.name));
                for func in &module.functions {
                    lines.push(format!(
                        "  {}/{} [{}] ({})",
                        func.name, func.arity, func.kind, func.file
                    ));
                    for clause in &func.clauses {
                        let pattern_str = if clause.pattern.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", clause.pattern)
                        };
                        let guard_str = if clause.guard.is_empty() {
                            String::new()
                        } else {
                            format!(" when {}", clause.guard)
                        };
                        lines.push(format!(
                            "    L{}:{}{}{}",
                            clause.start_line, clause.end_line, pattern_str, guard_str
                        ));
                    }
                }
            }
        } else {
            lines.push("No locations found.".to_string());
        }

        lines.join("\n")
    }
}
