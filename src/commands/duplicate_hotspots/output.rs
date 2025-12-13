use crate::output::Outputable;

use super::execute::DuplicateHotspotsResult;

impl Outputable for DuplicateHotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push("Modules with Most Duplicates".to_string());
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No duplicate functions found.".to_string());
        } else {
            lines.push(format!(
                "Found {} duplicated function(s) across {} module(s):",
                self.total_duplicates, self.total_modules
            ));
            lines.push(String::new());

            for module in &self.modules {
                lines.push(format!("{} ({} duplicates):", module.name, module.duplicate_count));

                for dup in &module.top_duplicates {
                    lines.push(format!(
                        "  {}/{} ({} copies)",
                        dup.name, dup.arity, dup.copy_count
                    ));
                }
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }
}
